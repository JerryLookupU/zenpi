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

class DozeLifecycleTest {
    @get:Rule val compose = createComposeRule()
    @Test fun deepIdleKeepsDurableCacheAndForegroundReauthenticates(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        assertEquals("true", args.getString("doze_validation"))
        val namespace = requireNotNull(args.getString("process_namespace"))
        val context = instrumentation.targetContext
        val store = PairedHostStore(context, namespace)
        val identity = requireNotNull(DeviceIdentity.loadExisting("zenpi.network-process.$namespace"))
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        fun shell(command: String): String = instrumentation.uiAutomation.executeShellCommand(command).use {
            android.os.ParcelFileDescriptor.AutoCloseInputStream(it).bufferedReader().use { reader -> reader.readText().trim() }
        }
        try {
            repeat(40) { if (model.state.value.host == null) delay(100) }
            compose.runOnUiThread { model.setForeground(true) }
            repeat(60) { if (model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            val live = requireNotNull(model.state.value.projection)
            val before = requireNotNull(store.loadState())
            compose.runOnUiThread { model.setForeground(false) }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            shell("dumpsys battery unplug")
            shell("input keyevent KEYCODE_SLEEP")
            val forced = shell("dumpsys deviceidle force-idle deep")
            assertEquals("Device must actually enter deep idle: $forced", "IDLE", shell("dumpsys deviceidle get deep"))
            delay(1500)
            assertEquals(before, store.loadState())
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(model.state.value.pendingWorkspace)
            shell("dumpsys deviceidle unforce")
            shell("dumpsys battery reset")
            shell("input keyevent KEYCODE_WAKEUP")
            shell("wm dismiss-keyguard")
            compose.runOnUiThread { model.setForeground(true) }
            repeat(100) { if (model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertEquals(before.host, store.loadState()?.host)
            assertEquals(before.projection?.sessionId, model.state.value.projection?.sessionId)
            assertEquals(live.turns, model.state.value.projection?.turns)
            assertEquals(before.projection, store.loadState()?.projection)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "doze_foreground_resume_verified") })
        } finally {
            shell("dumpsys deviceidle unforce"); shell("dumpsys battery reset")
            shell("input keyevent KEYCODE_WAKEUP"); shell("wm dismiss-keyguard")
            compose.runOnUiThread { model.setForeground(false); model.cancel() }
        }
    }
}
