package zenpi.security

import android.content.Context
import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import android.util.AtomicFile
import com.google.gson.JsonObject
import zenpi.transport.FrameCodec
import zenpi.transport.HostProjection
import zenpi.transport.WireException
import zenpi.transport.WorkspaceCommand
import zenpi.pairing.AddressHint
import zenpi.state.ProjectionCacheCodec
import zenpi.state.ProjectionCommitPolicy
import java.io.File
import java.security.KeyStore
import java.security.MessageDigest
import javax.crypto.Cipher
import javax.crypto.KeyGenerator
import javax.crypto.SecretKey
import javax.crypto.spec.GCMParameterSpec

/**
 * [fingerprint] pins the host: its key id `sha256(SPKI)`, or for a host saved before
 * ZE-515 ([legacyPin]) the certificate hash, migrated to the key id on next connect.
 */
data class PairedHost(val host: String, val port: Int, val fingerprint: String, val deviceId: String, val legacyPin: Boolean = false,
    /** The QR's later hints, in order, dialed after [host]:[port] fails to connect (ZE-516). */
    val fallbacks: List<AddressHint> = emptyList()) {
    val hints: List<AddressHint> get() = listOf(AddressHint(
        if (runCatching { zenpi.lifecycle.LanRoutePolicy.overlay(java.net.InetAddress.getByName(host)) }.getOrDefault(false)) "tailnet" else "lan",
        host, port)) + fallbacks
}
data class SavedHostState(val host: PairedHost, val projection: HostProjection?, val pendingWorkspace: WorkspaceCommand? = null, val pendingRevoke: String? = null)
data class ResumeSnapshot internal constructor(val state: SavedHostState, internal val stamp: String)

/** Encrypted no-backup host/display/cursor record; no QR token or private key. */
class PairedHostStore(context: Context, namespace: String = "v1") {
    init { require(namespace.matches(Regex("[A-Za-z0-9._-]{1,80}"))) }
    private val file = AtomicFile(File(context.noBackupFilesDir, "paired-host.$namespace"))
    private val reset = LocalResetFiles(file.baseFile)
    private val alias = "zenpi.host-metadata.$namespace"
    private fun store() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private fun key(create: Boolean): SecretKey {
        val store = store()
        if (!store.containsAlias(alias)) {
            check(create) { "Pairing encryption key unavailable" }
            KeyGenerator.getInstance(KeyProperties.KEY_ALGORITHM_AES, "AndroidKeyStore").apply {
                init(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_ENCRYPT or KeyProperties.PURPOSE_DECRYPT)
                    .setBlockModes(KeyProperties.BLOCK_MODE_GCM).setEncryptionPaddings(KeyProperties.ENCRYPTION_PADDING_NONE)
                    .setRandomizedEncryptionRequired(true).build())
            }.generateKey()
        }
        return store.getKey(alias, null) as SecretKey
    }
    companion object { private val storageLock = Any(); private const val MAX_BYTES = 196_608 }
    fun save(host: PairedHost, checkActive: () -> Unit = {}) = synchronized(storageLock) {
        checkActive()
        val next = RePairPolicy.state(read(), host)
        write(host, next.projection?.let(ProjectionCacheCodec::encode), null, checkActive = checkActive)
        next
    }
    /** Re-pin a v1-saved host to its key id, keeping display, cursors and pending requests. */
    fun migratePin(host: PairedHost, hostId: String, checkActive: () -> Unit = {}): PairedHost = synchronized(storageLock) {
        checkActive()
        require(host.legacyPin && hostId.matches(Regex("[0-9a-f]{64}")))
        val saved = requireHost(host)
        val migrated = host.copy(fingerprint = hostId, legacyPin = false)
        write(migrated, saved.projection?.let(ProjectionCacheCodec::encode), saved.pendingWorkspace, saved.pendingRevoke, checkActive)
        migrated
    }
    fun saveProjection(host: PairedHost, projection: HostProjection, checkActive: () -> Unit = {}) = synchronized(storageLock) {
        checkActive()
        val saved = requireHost(host)
        check(saved.pendingRevoke == null) { "Host revocation is unresolved" }
        ProjectionCommitPolicy.requireCurrent(saved.projection, projection)
        write(host, if (projection.selectedId == null) null else ProjectionCacheCodec.encode(projection), saved.pendingWorkspace,
            checkActive = checkActive)
    }
    private fun requireHost(host: PairedHost): SavedHostState = requireNotNull(read()).also {
        check(it.host == host) { "Saved host changed before cache commit" }
    }
    fun beginWorkspace(host: PairedHost, command: WorkspaceCommand, checkActive: () -> Unit = {}) = synchronized(storageLock) {
        checkActive()
        val saved = requireHost(host)
        check(saved.pendingRevoke == null) { "Host revocation is unresolved" }
        check(saved.pendingWorkspace == null || saved.pendingWorkspace.envelope == command.envelope) { "Another workspace request is unresolved" }
        write(host, saved.projection?.let(ProjectionCacheCodec::encode), command, checkActive = checkActive)
    }
    fun finishWorkspace(host: PairedHost, command: WorkspaceCommand, projection: HostProjection? = null,
        checkActive: () -> Unit = {}) = synchronized(storageLock) {
        checkActive()
        val saved = requireHost(host)
        check(saved.pendingRevoke == null) { "Host revocation is unresolved" }
        check(saved.pendingWorkspace?.envelope == command.envelope) { "Pending workspace request changed" }
        val display = projection ?: saved.projection
        display?.let { ProjectionCommitPolicy.requireCurrent(saved.projection, it) }
        write(host, display?.takeIf { it.selectedId != null }?.let(ProjectionCacheCodec::encode), null, checkActive = checkActive)
    }
    fun beginRevoke(host: PairedHost, requestId: String, checkActive: () -> Unit = {}) = synchronized(storageLock) {
        checkActive()
        require(java.util.UUID.fromString(requestId).toString() == requestId)
        val saved = requireHost(host)
        check(saved.pendingWorkspace == null && saved.pendingRevoke == null) { "Another request is unresolved" }
        write(host, saved.projection?.let(ProjectionCacheCodec::encode), null, requestId, checkActive)
    }
    fun finishRevoke(host: PairedHost, requestId: String, checkActive: () -> Unit = {}, removeSigningKey: () -> Unit) = synchronized(storageLock) {
        checkActive()
        val saved = requireHost(host)
        check(saved.pendingRevoke == requestId && saved.pendingWorkspace == null) { "Revocation intent changed" }
        // Retain the marker if key removal fails, rather than silently resuming.
        checkActive()
        removeSigningKey()
        wipe()
    }
    private fun write(host: PairedHost, projection: JsonObject?, pending: WorkspaceCommand?, pendingRevoke: String? = null,
        checkActive: () -> Unit = {}) {
        reset.requireAvailable()
        require(host.port in 1..65535 && validLiteralHost(host.host) && host.fingerprint.matches(Regex("[0-9a-f]{64}")) && host.deviceId.matches(Regex("[0-9a-f]{64}"))
            && validFallbacks(host))
        val value = JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", "local-metadata")
            addProperty("execution_state", "succeeded")
            add("payload", JsonObject().apply {
                addProperty("type", "response"); add("result", JsonObject().apply {
                    addProperty("host", host.host); addProperty("port", host.port)
                    addProperty("fingerprint", host.fingerprint); addProperty("device_id", host.deviceId)
                    if (!host.legacyPin) addProperty("pin", "host_id")
                    if (host.fallbacks.isNotEmpty()) add("fallbacks", com.google.gson.JsonArray().apply { host.fallbacks.forEach { hint ->
                        add(JsonObject().apply { addProperty("kind", hint.kind); addProperty("host", hint.host); addProperty("port", hint.port) })
                    } })
                    projection?.let { add("projection", it) }
                    pending?.let { add("pending_workspace", WorkspaceCommand.restore(it.envelope).envelope) }
                    pendingRevoke?.let { addProperty("pending_revoke", it) }
                })
            })
        }
        // The encrypted record must pass the same framing/complexity gates used on restore.
        val plaintext = FrameCodec.encode(value)
        check(plaintext.size < MAX_BYTES - 30) { "Host cache too large" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE, key(true)); cipher.updateAAD("zenpi-paired-host-v1".toByteArray())
        val bytes = byteArrayOf(1) + cipher.iv + cipher.doFinal(plaintext)
        checkActive()
        val output = file.startWrite()
        try { output.write(bytes); file.finishWrite(output) } catch (error: Exception) { file.failWrite(output); throw error }
    }
    fun load(): PairedHost? = synchronized(storageLock) { read()?.host }
    fun loadState(): SavedHostState? = synchronized(storageLock) { read() }
    fun loadResumeSnapshot(): ResumeSnapshot? = synchronized(storageLock) {
        val bytes = readBytes() ?: return@synchronized null
        val state = decode(bytes)
        if (state.pendingWorkspace != null || state.pendingRevoke != null) null else ResumeSnapshot(state, stamp(bytes))
    }
    fun commitResume(expected: ResumeSnapshot, projection: HostProjection, checkActive: () -> Unit = {}): Boolean = synchronized(storageLock) {
        // Acquiring this monitor can outlive worker cancellation; check under
        // ownership before reading or starting the non-suspending file commit.
        checkActive()
        val bytes = readBytes() ?: return@synchronized false
        // Any foreground commit, new pending write, re-pair or wipe invalidates this read.
        if (stamp(bytes) != expected.stamp || expected.state.pendingWorkspace != null || expected.state.pendingRevoke != null) return@synchronized false
        ProjectionCommitPolicy.requireCurrent(expected.state.projection, projection)
        write(expected.state.host, projection.takeIf { it.selectedId != null }?.let(ProjectionCacheCodec::encode), null,
            checkActive = checkActive)
        true
    }
    private fun stamp(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it.toInt() and 255) }
    private fun read(): SavedHostState? {
        return readBytes()?.let(::decode)
    }
    private fun readBytes(): ByteArray? {
        reset.requireAvailable()
        // openRead also recovers legacy AtomicFile backup-only interrupted writes.
        if (!file.baseFile.exists() && !File(file.baseFile.path + ".bak").exists()) return null
        return file.openRead().use { input ->
            val output = java.io.ByteArrayOutputStream()
            val buffer = ByteArray(512)
            while (output.size() <= MAX_BYTES) {
                val read = input.read(buffer, 0, minOf(buffer.size, MAX_BYTES + 1 - output.size()))
                if (read == -1) break
                output.write(buffer, 0, read)
            }
            output.toByteArray()
        }
    }
    private fun decode(bytes: ByteArray): SavedHostState {
        check(bytes.size in 30..MAX_BYTES && bytes[0] == 1.toByte()) { "Pairing metadata unavailable" }
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.DECRYPT_MODE, key(false), GCMParameterSpec(128, bytes.copyOfRange(1, 13)))
        cipher.updateAAD("zenpi-paired-host-v1".toByteArray())
        val frame = FrameCodec.decode(cipher.doFinal(bytes.copyOfRange(13, bytes.size)))
        if (frame.keySet() != setOf("schema_version", "channel", "id", "execution_state", "payload")
            || FrameCodec.string(frame, "channel") != "control"
            || FrameCodec.string(frame, "execution_state") != "succeeded") throw WireException("Invalid encrypted host metadata envelope")
        val payload = frame["payload"]?.asJsonObject
        if (payload == null || payload.keySet() != setOf("type", "result") || FrameCodec.string(payload, "type") != "response")
            throw WireException("Invalid encrypted host metadata payload")
        val result = payload["result"]?.asJsonObject ?: throw WireException("Missing encrypted host metadata result")
        check(result.keySet().containsAll(setOf("host", "port", "fingerprint", "device_id"))
            && result.keySet().all { it in setOf("host", "port", "fingerprint", "device_id", "pin", "fallbacks", "projection", "pending_workspace", "pending_revoke") })
        // Records written before ZE-515 carry no pin kind: they pin the certificate hash.
        val legacyPin = !result.has("pin")
        check(legacyPin || FrameCodec.string(result, "pin") == "host_id")
        val portValue = result["port"]
        check(portValue.isJsonPrimitive && portValue.asJsonPrimitive.isNumber && portValue.toString().matches(Regex("[1-9][0-9]{0,4}")))
        val port = portValue.asInt
        val primary = FrameCodec.string(result, "host")
        // Records written before ZE-516 hold only the primary address.
        val fallbacks = result["fallbacks"]?.let { value ->
            check(value.isJsonArray && value.asJsonArray.size() in 1..3)
            value.asJsonArray.map { entry ->
                val hint = entry.asJsonObject
                check(hint.keySet() == setOf("kind", "host", "port") && hint["port"].asJsonPrimitive.isNumber
                    && hint["port"].toString().matches(Regex("[1-9][0-9]{0,4}")))
                AddressHint(FrameCodec.string(hint, "kind"), FrameCodec.string(hint, "host"), hint["port"].asInt)
            }
        } ?: emptyList()
        val host = PairedHost(primary, port, FrameCodec.string(result, "fingerprint"), FrameCodec.string(result, "device_id"), legacyPin, fallbacks)
        check(port in 1..65535 && validLiteralHost(host.host) && host.fingerprint.matches(Regex("[0-9a-f]{64}")) && host.deviceId.matches(Regex("[0-9a-f]{64}"))
            && validFallbacks(host))
        val pendingRevoke = if (result.has("pending_revoke")) FrameCodec.string(result, "pending_revoke").also {
            check(java.util.UUID.fromString(it).toString() == it && !result.has("pending_workspace"))
        } else null
        val projection = result["projection"]?.let {
            if (!it.isJsonObject) throw WireException("Invalid cached host projection")
            ProjectionCacheCodec.decode(it.asJsonObject)
        }
        val pending = result["pending_workspace"]?.let {
            if (!it.isJsonObject) throw WireException("Invalid pending workspace command")
            WorkspaceCommand.restore(it.asJsonObject)
        }
        return SavedHostState(host, projection, pending, pendingRevoke)
    }
    fun beginLocalReset() = synchronized(storageLock) { reset.begin() }
    fun wipe() = synchronized(storageLock) {
        reset.begin()
        val keys = store()
        keys.deleteEntry(alias)
        check(!keys.containsAlias(alias)) { "Metadata key removal failed" }
        file.delete()
        reset.finish()
    }

    private fun validFallbacks(host: PairedHost): Boolean = host.fallbacks.size <= 3
        && host.fallbacks.all { it.kind in AddressHint.KINDS && it.port in 1..65535 && validLiteralHost(it.host) }
        && host.hints.map { it.address }.toSet().size == host.hints.size

    private fun validLiteralHost(host: String): Boolean {
        if (host.isEmpty() || host.length > 128) return false
        // Reject DNS names and scoped spellings before touching InetAddress.
        // getByName performs resolver work even when the eventual shape check
        // would reject the input, which is undesirable on the pairing path.
        val shape = if (host.contains(':')) host.matches(Regex("[a-fA-F0-9:.]+"))
        else host.split('.').size == 4 && host.split('.').all {
            it.matches(Regex("0|[1-9][0-9]{0,2}")) && it.toInt() <= 255
        }
        if (!shape || host.contains('%')) return false
        val parsed = runCatching { java.net.InetAddress.getByName(host) }.getOrNull() ?: return false
        // Match Rust's invite validator: IPv4-mapped IPv6 is checked as the
        // underlying IPv4 address so mapped broadcast/link-local targets cannot
        // bypass the destination policy.
        val address = parsed.address.takeIf { bytes ->
            bytes.size == 16 && bytes.take(10).all { it == 0.toByte() }
                && bytes[10] == 0xff.toByte() && bytes[11] == 0xff.toByte()
        }?.let { bytes -> java.net.InetAddress.getByAddress(bytes.copyOfRange(12, 16)) } ?: parsed
        // Loopback is deliberately retained for adb-reverse/local fixtures;
        // unspecified addresses remain rejected by isAnyLocalAddress.
        return (!address.isAnyLocalAddress || address.isLoopbackAddress) && !address.isMulticastAddress
            && !address.isLinkLocalAddress && address.hostAddress != "255.255.255.255"
    }
}
