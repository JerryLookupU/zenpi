package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import kotlinx.coroutines.CompletableDeferred
import kotlinx.coroutines.coroutineScope
import kotlinx.coroutines.delay
import kotlinx.coroutines.launch
import kotlinx.coroutines.runBlocking
import kotlinx.coroutines.withTimeout
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.PairingInvite
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHost
import zenpi.transport.*
import java.io.File
import java.security.Key
import java.security.KeyPair
import java.security.KeyPairGenerator
import java.security.KeyStoreException
import java.security.KeyStoreSpi
import java.security.Provider
import java.security.PublicKey
import java.security.Security
import java.security.SignatureException
import java.security.cert.Certificate
import java.security.spec.ECGenParameterSpec
import java.util.Collections
import java.util.Date
import java.util.Enumeration
import java.util.concurrent.ConcurrentHashMap

/**
 * Software stand-in for AndroidKeyStore on a plain JVM: the production
 * [DeviceIdentity] signs with a P-256 key held only in this process.
 */
class SoftwareKeyStoreSpi : KeyStoreSpi() {
    companion object { val keys = ConcurrentHashMap<String, KeyPair>() }
    private class PublicKeyCertificate(private val key: PublicKey) : Certificate("X.509") {
        override fun getEncoded(): ByteArray = key.encoded
        override fun verify(key: PublicKey) { throw SignatureException("Unsigned test certificate") }
        override fun verify(key: PublicKey, sigProvider: String) { throw SignatureException("Unsigned test certificate") }
        override fun toString() = "PublicKeyCertificate"
        override fun getPublicKey(): PublicKey = key
    }
    override fun engineGetKey(alias: String, password: CharArray?): Key? = keys[alias]?.private
    override fun engineGetCertificateChain(alias: String): Array<Certificate>? = engineGetCertificate(alias)?.let { arrayOf(it) }
    override fun engineGetCertificate(alias: String): Certificate? = keys[alias]?.let { PublicKeyCertificate(it.public) }
    override fun engineGetCreationDate(alias: String): Date? = null
    override fun engineSetKeyEntry(alias: String, key: Key, password: CharArray?, chain: Array<out Certificate>?) { throw KeyStoreException("Read only") }
    override fun engineSetKeyEntry(alias: String, key: ByteArray, chain: Array<out Certificate>?) { throw KeyStoreException("Read only") }
    override fun engineSetCertificateEntry(alias: String, cert: Certificate) { throw KeyStoreException("Read only") }
    override fun engineDeleteEntry(alias: String) { keys.remove(alias) }
    override fun engineAliases(): Enumeration<String> = Collections.enumeration(keys.keys)
    override fun engineContainsAlias(alias: String) = keys.containsKey(alias)
    override fun engineSize() = keys.size
    override fun engineIsKeyEntry(alias: String) = keys.containsKey(alias)
    override fun engineIsCertificateEntry(alias: String) = false
    override fun engineGetCertificateAlias(cert: Certificate): String? = null
    override fun engineStore(stream: java.io.OutputStream?, password: CharArray?) { throw KeyStoreException("Read only") }
    override fun engineLoad(stream: java.io.InputStream?, password: CharArray?) {}
}

/**
 * ZE-590, invoked by tests/remote_host_e2e.rs against a running `zenpi host serve`:
 * the production Kotlin client pairs over real TCP, prompts, streams, approves,
 * disconnects while the host finishes, and replays from its held cursor.
 */
class AndroidRustHostE2eTest {
    private fun property(name: String) = requireNotNull(System.getProperty("zenpi.e2e.$name")) { name }
    private fun HostProjection.project() = projects.single { it.id == selectedId }

    private fun softwareIdentity(alias: String): DeviceIdentity {
        if (Security.getProvider("AndroidKeyStore") == null)
            @Suppress("DEPRECATION")
            Security.addProvider(object : Provider("AndroidKeyStore", 1.0, "JVM software keystore") {
                init { put("KeyStore.AndroidKeyStore", SoftwareKeyStoreSpi::class.java.name) }
            })
        SoftwareKeyStoreSpi.keys[alias] = KeyPairGenerator.getInstance("EC")
            .apply { initialize(ECGenParameterSpec("secp256r1")) }.generateKeyPair()
        return requireNotNull(DeviceIdentity.loadExisting(alias))
    }

    /**
     * One subscription until [done], running [meanwhile] once it is open; cancelling it
     * closes the TLS socket (a real disconnect).
     */
    private suspend fun subscribeUntil(client: HostClient, host: PairedHost, identity: DeviceIdentity, from: HostProjection,
        observe: (HostProjection) -> Unit, done: (HostProjection) -> Boolean,
        meanwhile: suspend () -> Unit = {}): HostProjection = coroutineScope {
        val opened = CompletableDeferred<Unit>()
        val reached = CompletableDeferred<HostProjection>()
        val subscription = launch {
            client.inspect(host, from.selectedId, identity, previous = from, subscribe = true, onProjection = {
                observe(it)
                opened.complete(Unit)
                if (done(it)) reached.complete(it)
            })
        }
        try { withTimeout(30_000) { opened.await(); meanwhile(); reached.await() } }
        finally { subscription.cancel(); subscription.join() }
    }

    @Test fun productionHostPairPromptStreamApprovalDisconnectReplay() = runBlocking {
        assumeNotNull(System.getProperty("zenpi.e2e.invite"))
        val prompt = property("prompt"); val stream = property("stream"); val final = property("final")
        val workspace = File(property("workspace")); val journal = File(property("journal"))
        val identity = softwareIdentity("zenpi.e2e.device")

        // Pair: the production PairingClient over pinned TLS on real TCP.
        val invite = PairingInvite.parse(property("invite"), monotonicNow = System::nanoTime)
        val paired = PairingClient().pair(invite, "jvm-phone", identity)
        assertTrue(paired.controlAvailable)
        val host = PairedHost(invite.host, invite.port, invite.fingerprint, paired.deviceId)
        val client = HostClient()

        // Prompt, sent on its own connection while the phone is subscribed, so the
        // output reaches the phone as live subscription deltas.
        val opened = client.inspect(host, identity = identity)
        assertTrue(opened.project().promptControl)
        val command = WorkspaceCommand.prompt(opened, prompt)
        val requestId = command.envelope["id"].asString
        fun streamText(projection: HostProjection) = projection.project().promptStatuses.singleOrNull { it.requestId == requestId }?.streamText

        // Streaming output, up to the approval the tool call waits for.
        val streamed = mutableListOf<String>()
        var views = 0
        val pending = subscribeUntil(client, host, identity, opened,
            observe = { view ->
                if (views++ == 0) assertNull("nothing streams before the prompt", streamText(view))
                streamText(view)?.let { if (streamed.lastOrNull() != it) streamed += it }
            },
            done = { it.project().pendingApprovals.isNotEmpty() && streamText(it) == stream },
            meanwhile = { client.inspect(host, opened.selectedId, identity, previous = opened, workspaceCommand = command) })
        assertTrue(views > 1)
        assertEquals(stream, streamed.last())
        for ((before, after) in streamed.zipWithNext()) assertTrue("$before -> $after", after.startsWith(before))
        assertFalse("tool must wait for the phone", File(workspace, "approved.txt").exists())

        // Approval, on its own connection; the subscription above is already closed.
        val approval = pending.project().pendingApprovals.single()
        val approved = client.inspect(host, pending.selectedId, identity, previous = pending,
            workspaceCommand = WorkspaceCommand.approve(pending, approval, "allow"))

        // Disconnected: the host runs the tool and the final answer while no client is attached.
        val deadline = System.nanoTime() + 20_000_000_000
        while (!journal.readText().contains(final)) {
            assertTrue("host never finished offline", System.nanoTime() < deadline)
            delay(20)
        }

        // Replay: reconnect resumes from the held cursor, then keeps subscribing.
        var resumed: HostProjection? = null
        val finished = subscribeUntil(client, host, identity, approved,
            observe = { if (resumed == null) resumed = it },
            done = { view -> view.project().promptStatuses.any { it.requestId == requestId && it.state == "succeeded" } })
        val first = requireNotNull(resumed)
        assertTrue("offline turns arrive by replay", first.replayedEventCount > approved.replayedEventCount)
        assertTrue(first.turns.any { it.role == "assistant" && it.content == final })

        val turns = finished.turns
        assertFalse(finished.truncated)
        assertEquals("duplicate turn", turns.size, turns.map { it.id }.distinct().size)
        assertEquals(1, turns.count { it.role == "user" && it.content == prompt })
        assertEquals(1, turns.count { it.role == "tool" })
        assertEquals(1, turns.count { it.role == "assistant" && it.content == final })
        File(property("output")).writeText(JsonObject().apply {
            add("turns", JsonArray().apply {
                turns.forEach { turn -> add(JsonObject().apply {
                    addProperty("id", turn.id); addProperty("role", turn.role); addProperty("content", turn.content)
                }) }
            })
            add("streamed", JsonArray().apply { streamed.forEach(::add) })
            addProperty("event_sequence", finished.eventSequence)
            addProperty("device_id", paired.deviceId)
        }.toString())
    }
}
