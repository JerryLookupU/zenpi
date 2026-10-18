package zenpi

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import zenpi.transport.*

class DeviceIdentityAdmissionTest {
    @get:Rule val compose = createComposeRule()

    @Test fun wipedOrMismatchedSigningKeyFencesForegroundWithoutReplacingIdentity(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        for (wiped in listOf(true, false)) {
            val namespace = "identity-admission-${System.nanoTime()}"
            val alias = "zenpi.$namespace"
            val identity = DeviceIdentity.loadOrCreate(alias)
            val store = PairedHostStore(context, namespace)
            val host = PairedHost("192.168.50.55", 7777, "a".repeat(64),
                if (wiped) identity.deviceId else "b".repeat(64))
            val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")),
                "project", "session", "idle", listOf(RemoteTurn("turn", "assistant", "durable display")),
                "1", false, eventSequence = "0")
            store.save(host); store.saveProjection(host, projection)
            val saved = requireNotNull(store.loadState())
            lateinit var model: PairingViewModel
            compose.runOnUiThread {
                model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
            }
            try {
                repeat(60) { if (model.state.value.host == null) delay(50) }
                assertEquals(host, model.state.value.host)
                if (wiped) identity.wipe()
                compose.runOnUiThread { model.setForeground(true) }
                repeat(60) { if (model.state.value.phase != PairingPhase.ERROR) delay(50) }
                val fenced = model.state.value
                assertEquals(PairingPhase.ERROR, fenced.phase)
                assertEquals("Host access or ownership changed. Refresh to authenticate again.", fenced.error)
                assertEquals(host, fenced.host); assertTrue(fenced.registered)
                assertEquals(saved.projection, fenced.projection)
                assertNull(fenced.pendingWorkspace); assertNull(fenced.projection?.workspaceOwner)
                assertEquals(saved, store.loadState())
                delay(1_200)
                assertEquals(fenced, model.state.value)
                if (wiped) assertNull(DeviceIdentity.loadExisting(alias))
                else assertNotEquals(host.deviceId, identity.deviceId)
            } finally {
                compose.runOnUiThread { model.setForeground(false); model.cancel() }
                store.wipe(); identity.wipe()
            }
        }
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_identity_admission", "wiped_mismatched_key_foreground_fenced_verified")
        })
    }
}
