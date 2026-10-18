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

class ForegroundReadTest {
    @get:Rule val compose = createComposeRule()

    @Test fun backgroundRefreshCannotStartConnectionOrChangeEncryptedOfflineDisplay(): Unit = runBlocking {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val namespace = "read-fence-${System.nanoTime()}"
        val store = PairedHostStore(context, namespace)
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), "b".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", diffAvailable = true)),
            "project", "session", "idle", listOf(RemoteTurn("turn", "assistant", "cached display")), "1", false, "1")
        store.save(host); store.saveProjection(host, projection)
        val identity = DeviceIdentity.loadOrCreate("zenpi.$namespace")
        lateinit var model: PairingViewModel
        try {
            compose.runOnUiThread { model = PairingViewModel(context.applicationContext as android.app.Application, store, identity) }
            repeat(60) { if (model.state.value.host == null) delay(100) }
            assertEquals(host, model.state.value.host)
            val before = model.state.value
            val file = java.io.File(context.noBackupFilesDir, "paired-host.$namespace")
            val bytes = file.readBytes()
            compose.runOnUiThread {
                model.refresh(); model.refresh("project"); model.refresh("foreign")
                model.reviewDiff("."); model.reviewDiff("cached.txt")
                model.readTranscript("turn"); model.readTranscript("turn", true)
                model.previousTranscript()
                model.browseFiles(); model.browseFiles("docs")
            }
            assertEquals(before, model.state.value)
            delay(500)
            assertEquals(before, model.state.value); assertArrayEquals(bytes, file.readBytes())
            assertNull(model.state.value.diff); assertFalse(model.state.value.diffLoading)
            compose.runOnUiThread { model.cancel() }
            val stopped = model.state.value
            assertEquals(PairingPhase.OFFLINE, stopped.phase); assertEquals(host, stopped.host)
            assertEquals(before.projection, stopped.projection); assertTrue(stopped.registered)
            assertArrayEquals(bytes, file.readBytes())
            compose.runOnUiThread { model.refresh() }
            assertEquals(stopped, model.state.value)
        } finally {
            compose.runOnUiThread { model.setForeground(false); model.cancel() }
            store.wipe(); identity.wipe()
        }
    }
}
