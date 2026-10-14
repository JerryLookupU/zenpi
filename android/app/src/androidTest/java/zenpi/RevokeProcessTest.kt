package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*

class RevokeProcessTest {
    @Test fun unresolvedRevokeSurvivesIndependentProcessWithoutResumeAdmission() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val phase = args.getString("cache_phase") ?: return
        val namespace = requireNotNull(args.getString("cache_namespace"))
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        val alias = "zenpi.revoke.$namespace"
        val identity = DeviceIdentity.loadOrCreate(alias)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), identity.deviceId)
        if (phase == "seed") {
            store.wipe(); store.save(host); store.beginRevoke(host, java.util.UUID.randomUUID().toString())
        } else {
            assertEquals("restore", phase)
            assertNotEquals(requireNotNull(args.getString("previous_pid")).toInt(), android.os.Process.myPid())
        }
        val intent = requireNotNull(store.loadState()?.pendingRevoke)
        if (phase == "restore") assertEquals(args.getString("revoke_intent"), intent)
        try {
            assertEquals(host, store.load())
            assertNotNull(store.loadState()?.pendingRevoke)
            assertNull(store.loadResumeSnapshot())
            assertTrue(runCatching { store.save(host) }.isFailure)
            assertEquals(host.deviceId, DeviceIdentity.loadExisting(alias)?.deviceId)
        } finally { if (phase == "restore") { store.wipe(); identity.wipe() } }
        instrumentation.sendStatus(0, android.os.Bundle().apply {
            putString("cache_process_pid", android.os.Process.myPid().toString())
            putString("cache_process_phase", phase)
            putString("revoke_intent", intent)
        })
    }
}
