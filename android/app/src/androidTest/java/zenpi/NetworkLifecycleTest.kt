package zenpi

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.lifecycle.NetworkMonitor
import zenpi.pairing.*
import zenpi.security.*

class NetworkLifecycleTest {
    @get:Rule val compose = createComposeRule()
    @Test fun defaultNetworkLossKeepsCacheOfflineAndRecoveryResumesWithoutRescan() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val namespace = requireNotNull(InstrumentationRegistry.getArguments().getString("process_namespace"))
        val context = instrumentation.targetContext
        val store = PairedHostStore(context, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.network-process.$namespace")
        val model = PairingViewModel(context.applicationContext as android.app.Application, store, identity)
        val route = java.util.concurrent.atomic.AtomicReference<Long?>()
        val monitor = NetworkMonitor(context) { route.set(it); model.networkChanged(it) }
        fun readOutput(descriptor: android.os.ParcelFileDescriptor): String =
            android.os.ParcelFileDescriptor.AutoCloseInputStream(descriptor).bufferedReader().use { it.readText().trim() }
        fun shell(command: String): String {
            if (android.os.Build.VERSION.SDK_INT < 34)
                return readOutput(instrumentation.uiAutomation.executeShellCommand(command))
            // API 34's Wi-Fi permission refusal is written to stderr. Ignoring
            // that descriptor falsely treats a rejected association as success.
            val pipes = instrumentation.uiAutomation.executeShellCommandRwe(command)
            pipes[1].close()
            return readOutput(pipes[0]) + "\n" + readOutput(pipes[2])
        }
        val wifi = shell("settings get global wifi_on").trim() == "1"
        val data = shell("settings get global mobile_data").trim() == "1"
        val emulatorWifi = shell("getprop ro.kernel.qemu").trim() == "1"
            && shell("cmd wifi status").contains("Wifi is connected to \"AndroidWifi\"")
        fun restoreNetwork() {
            shell("svc wifi ${if (wifi) "enable" else "disable"}")
            shell("svc data ${if (data) "enable" else "disable"}")
            if (wifi && emulatorWifi) {
                val association = shell("cmd wifi connect-network AndroidWifi open")
                val result = if (association.contains("SecurityException") && association.contains("Uid 2000 does not have access"))
                    shell("su 0 cmd wifi connect-network AndroidWifi open") else association
                assertFalse("Owned emulator Wi-Fi association failed: $result", result.contains("Exception"))
            }
        }
        try {
            repeat(30) { if (model.state.value.host == null) delay(100) }
            compose.runOnUiThread { monitor.start(); model.setForeground(true) }
            repeat(60) { if (route.get() == null || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertNotNull("A real LAN route is required", route.get())
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            val cached = requireNotNull(model.state.value.projection)
            val displayOnly = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(cached))
            val oldToken = route.get()
            compose.runOnUiThread { monitor.stop(); monitor.start() }
            assertNotNull(route.get()); assertNotEquals(oldToken, route.get())
            repeat(60) { if (model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertEquals(cached.sessionId, model.state.value.projection?.sessionId)
            shell("svc wifi disable"); shell("svc data disable")
            repeat(100) { if (route.get() != null || model.state.value.phase != PairingPhase.OFFLINE) delay(100) }
            val connectivity = context.getSystemService(android.net.ConnectivityManager::class.java)
            val routes = connectivity.allNetworks.joinToString { network ->
                "${network.networkHandle}:${connectivity.getNetworkCapabilities(network)}:${connectivity.getLinkProperties(network)}"
            }
            assertNull("ConnectivityManager must report LAN route loss; observed=$routes", route.get())
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertEquals(displayOnly, model.state.value.projection)
            assertEquals(cached.eventSequence, store.loadState()?.projection?.eventSequence)
            val offline = requireNotNull(model.state.value.projection)
            assertNull(offline.workspaceOwner); assertNull(offline.workspaceRevision)
            assertTrue(offline.projects.all { !it.promptControl && !it.approvalControl && !it.sessionControl && !it.terminalControl })
            assertTrue(offline.projects.all { it.pendingApprovals.isEmpty() && it.terminal == null && it.desktopStreamText == null })
            compose.runOnUiThread { model.refresh() }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            restoreNetwork()
            repeat(150) { if (route.get() == null || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertNotNull(route.get()); assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertEquals(cached.sessionId, model.state.value.projection?.sessionId)
            assertEquals(cached.eventSequence, model.state.value.projection?.eventSequence)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "default_network_loss_resume_verified") })
            val stoppedRoute = route.get()
            compose.runOnUiThread { model.setForeground(false); monitor.stop() }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(model.state.value.projection?.workspaceOwner)
            assertTrue(requireNotNull(model.state.value.projection).projects.all { !it.promptControl && !it.approvalControl })
            val stopped = model.state.value
            shell("svc wifi disable"); shell("svc data disable")
            repeat(20) { delay(100); assertEquals(stoppedRoute, route.get()); assertEquals(stopped, model.state.value) }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "stopped_network_observation_fenced_verified") })
        } finally {
            restoreNetwork()
            compose.runOnUiThread { monitor.stop(); model.setForeground(false); model.cancel() }
        }
    }
}
