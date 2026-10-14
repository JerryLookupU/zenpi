package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*
import java.io.File
import java.security.KeyStore
import javax.crypto.Cipher
import javax.crypto.SecretKey

class PairedHostMigrationTest {
    private val host = PairedHost("127.0.0.1",7777,"a".repeat(64),"b".repeat(64))
    private val context get() = InstrumentationRegistry.getInstrumentation().targetContext
    private val projection = HostProjection(listOf(RemoteProject("project","Workspace","session")),"project","session","idle",
        listOf(RemoteTurn("turn","assistant","cached answer")),"42",false,"7")
    private fun base(namespace: String) = File(context.noBackupFilesDir,"paired-host.$namespace")
    private fun keys() = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    private fun writeLegacy(namespace: String, cache: JsonObject, pending: WorkspaceCommand) {
        val record = JsonObject().apply {
            addProperty("schema_version",1); addProperty("channel","control"); addProperty("id","legacy-metadata"); addProperty("execution_state","succeeded")
            add("payload",JsonObject().apply { addProperty("type","response"); add("result",JsonObject().apply {
                addProperty("host",host.host);addProperty("port",host.port);addProperty("fingerprint",host.fingerprint);addProperty("device_id",host.deviceId)
                add("projection",cache);add("pending_workspace",pending.envelope)
            }) })
        }
        val key = keys().getKey("zenpi.host-metadata.$namespace",null) as SecretKey
        val cipher = Cipher.getInstance("AES/GCM/NoPadding")
        cipher.init(Cipher.ENCRYPT_MODE,key);cipher.updateAAD("zenpi-paired-host-v1".toByteArray())
        base(namespace).writeBytes(byteArrayOf(1)+cipher.iv+cipher.doFinal(record.toString().toByteArray(Charsets.UTF_8)))
    }
    @Test fun encryptedLegacyDisplaysUpgradeOnlyAfterConfirmedCommitAndRetainPendingIdentity() {
        val migrated = projection.copy(turns = projection.turns.map { it.copy(toolCallsOmitted = true) })
        for(version in listOf(1,2,3)) {
            val namespace="migration-$version-${System.nanoTime()}"
            val store=PairedHostStore(context,namespace)
            val command=WorkspaceCommand.create(projection.copy(projects=projection.projects.map{it.copy(workspaceControl=true)},
                workspaceOwner="c".repeat(64),workspaceRevision="3"),"project","workspace_select")
            try {
                store.save(host)
                val cache=ProjectionCacheCodec.encode(projection).apply {
                    addProperty("version",version)
                    if(version==1)remove("subtabs")
                    if(version<3)getAsJsonObject("listing").getAsJsonArray("projects").forEach{it.asJsonObject.remove("prompt_statuses")}
                }
                writeLegacy(namespace,cache,command)
                val original=base(namespace).readBytes()
                val restarted=PairedHostStore(context,namespace)
                val loaded=requireNotNull(restarted.loadState())
                assertEquals(host,loaded.host);assertEquals(migrated,loaded.projection)
                assertEquals(command.envelope,loaded.pendingWorkspace?.envelope)
                assertNull(restarted.loadResumeSnapshot())
                assertArrayEquals(original,base(namespace).readBytes())
                assertFalse(requireNotNull(loaded.projection).projects.single().workspaceControl)
                restarted.finishWorkspace(host,command)
                assertNull(store.loadState()?.pendingWorkspace)
                assertEquals(migrated,store.loadState()?.projection)
                assertFalse(original.contentEquals(base(namespace).readBytes()))
                assertEquals("4",ProjectionCacheCodec.encode(requireNotNull(store.loadState()?.projection))["version"].toString())
            } finally {store.wipe()}
        }
    }
    @Test fun restoredCiphertextWithoutItsKeystoreKeyIsPreservedUntilExplicitReset() {
        val namespace="missing-key-${System.nanoTime()}"
        val store=PairedHostStore(context,namespace)
        val alias="zenpi.host-metadata.$namespace"
        try {
            store.save(host);store.saveProjection(host,projection)
            val original=base(namespace).readBytes()
            keys().deleteEntry(alias)
            val restored=PairedHostStore(context,namespace)
            assertTrue(runCatching{restored.loadState()}.isFailure)
            assertTrue(runCatching{restored.loadResumeSnapshot()}.isFailure)
            assertTrue(runCatching{restored.save(host)}.isFailure)
            assertTrue(runCatching{restored.saveProjection(host,projection)}.isFailure)
            assertFalse(keys().containsAlias(alias))
            assertArrayEquals(original,base(namespace).readBytes())
            restored.wipe();assertNull(restored.loadState())
            restored.save(host);assertEquals(host,restored.load())
        } finally {store.wipe()}
    }
    @Test fun copiedOrTamperedEncryptedMetadataNeverBecomesANewPairing() {
        val first="copy-source-${System.nanoTime()}";val second="copy-target-${System.nanoTime()}"
        val source=PairedHostStore(context,first);val target=PairedHostStore(context,second)
        try {
            source.save(host);target.save(host.copy(port=8888))
            val original=base(first).readBytes()
            base(second).writeBytes(original)
            assertTrue(runCatching{target.loadState()}.isFailure)
            assertTrue(runCatching{target.save(host)}.isFailure)
            assertArrayEquals(original,base(second).readBytes())
            val tampered=original.clone().apply{this[lastIndex]=(this[lastIndex].toInt() xor 1).toByte()}
            base(first).writeBytes(tampered)
            assertTrue(runCatching{source.loadState()}.isFailure)
            assertArrayEquals(tampered,base(first).readBytes())
            target.wipe();source.wipe()
            assertNull(source.loadState());assertNull(target.loadState())
        } finally {source.wipe();target.wipe()}
    }
    @Test fun savedHostRejectsDnsAndUnsafeLiteralDestinationsBeforeWriting() {
        val namespace = "host-literal-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        try {
            for (value in listOf("example.invalid", "0.0.0.0", "169.254.1.2", "224.0.0.1", "255.255.255.255",
                "::ffff:169.254.1.2", "::ffff:255.255.255.255", "fd00::1%wlan0")) {
                assertTrue(value, runCatching { store.save(host.copy(host = value)) }.isFailure)
                assertFalse(base(namespace).exists())
            }
        } finally { store.wipe() }
    }
    @Test fun confirmedLocalWipeRemovesInterruptedFilesKeysAndPendingRequestsButNotOtherHosts() {
        val namespace="wipe-migration-${System.nanoTime()}";val otherNamespace="wipe-retained-${System.nanoTime()}"
        val store=PairedHostStore(context,namespace);val other=PairedHostStore(context,otherNamespace)
        val identity=DeviceIdentity.loadOrCreate("zenpi.$namespace")
        val command=WorkspaceCommand.create(projection.copy(projects=projection.projects.map{it.copy(workspaceControl=true)},
            workspaceOwner="c".repeat(64),workspaceRevision="3"),"project","workspace_select")
        try {
            store.save(host);store.saveProjection(host,projection);store.beginWorkspace(host,command)
            other.save(host.copy(port=8888))
            val retained=base(otherNamespace).readBytes()
            val encrypted=base(namespace).readBytes()
            File(base(namespace).path+".bak").writeBytes(encrypted)
            File(base(namespace).path+".new").writeBytes(byteArrayOf(1,2,3))
            store.beginLocalReset()
            assertTrue(runCatching{PairedHostStore(context,namespace).loadState()}.isFailure)
            identity.wipe();store.wipe()
            for(suffix in listOf("",".bak",".new",".reset"))assertFalse(File(base(namespace).path+suffix).exists())
            assertNull(PairedHostStore(context,namespace).loadState())
            assertNull(DeviceIdentity.loadExisting("zenpi.$namespace"))
            assertFalse(keys().containsAlias("zenpi.host-metadata.$namespace"))
            assertArrayEquals(retained,base(otherNamespace).readBytes())
            assertEquals(host.copy(port=8888),other.load())
        } finally {store.wipe();identity.wipe();other.wipe()}
    }
}
