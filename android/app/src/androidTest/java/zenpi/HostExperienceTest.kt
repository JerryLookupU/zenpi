package zenpi

import android.content.Intent
import android.os.SystemClock
import android.util.Log
import androidx.lifecycle.ViewModelProvider
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.runner.lifecycle.ActivityLifecycleMonitorRegistry
import androidx.test.runner.lifecycle.Stage
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Configurator
import androidx.test.uiautomator.UiDevice
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.AfterClass
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Assume.assumeTrue
import org.junit.FixMethodOrder
import org.junit.Test
import org.junit.runners.MethodSorters
import zenpi.lifecycle.ApprovalNotifier
import zenpi.lifecycle.LanSocketRouting
import zenpi.lifecycle.ResumeScheduler
import zenpi.pairing.PairingInvite
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingViewModel
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHost
import zenpi.security.PairedHostStore
import zenpi.transport.HostClient
import zenpi.transport.HostProjection
import zenpi.transport.HostRefusedException
import zenpi.transport.PairingClient
import zenpi.transport.RemoteApproval
import zenpi.transport.WorkspaceCommand
import zenpi.transport.WorkspaceUnknownOutcomeException

/**
 * ZE-591 against the production host: `zenpi host serve` with the scripted provider
 * (tests/ze591_android_host.rs, reached through `adb reverse`), passed as the `ze591_invite`
 * instrumentation argument; skipped without it. The device pairs into the production store and key
 * (what MainActivity and ResumeWorker use). In order:
 *  a_ with the app in the background, the background read (ResumeWorker) shows the approval
 *     notification within 2 s;
 *  b_ tapping the approval notification shows that session with its approval bar within 1 s;
 *  c_ after 30 s in airplane mode, while the desktop answered three prompts, the app reconnects with
 *     no duplicate and no missing turn, and the composer draft is still there.
 */
@FixMethodOrder(MethodSorters.NAME_ASCENDING)
class HostExperienceTest {
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val context = instrumentation.targetContext
    private val device = UiDevice.getInstance(instrumentation)
    private fun client() = HostClient(LanSocketRouting(context)::bind)

    companion object {
        private const val TITLE = "Approve write_file?"
        private var desktop: Desktop? = null

        @JvmStatic @AfterClass fun forget() {
            val context = InstrumentationRegistry.getInstrumentation().targetContext
            ResumeScheduler.cancel(context); ApprovalNotifier.cancel(context)
            PairedHostStore(context).wipe(); DeviceIdentity.loadExisting()?.wipe()
        }
    }

    /** Pairs once per run into the store and key MainActivity and ResumeWorker use. */
    private fun desktop(): Desktop = desktop ?: runBlocking {
        val uri = InstrumentationRegistry.getArguments().getString("ze591_invite")
        assumeTrue("needs the tests/ze591_android_host.rs host (ze591_invite argument)", uri != null)
        val identity = DeviceIdentity.loadOrCreate()
        val invite = PairingInvite.parse(requireNotNull(uri))
        val result = PairingClient(LanSocketRouting(context)::bind).pair(invite, "ZE-591 emulator", identity)
        val host = PairedHost(invite.host, invite.port, invite.fingerprint, result.deviceId)
        PairedHostStore(context).save(host)
        Desktop(host, identity).also { desktop = it }
    }

    /**
     * The desktop side of these scenarios: a second client of the host following it from its cursor.
     * Transcript reads need the session owner; the host refuses them as busy while a turn holds it.
     */
    private class Desktop(val host: PairedHost, val identity: DeviceIdentity) {
        private var last: HostProjection? = null
        private fun client() = HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)

        /**
         * Retries [call] while the host is transiently unavailable: a busy session owner, or the
         * listener's single unauthenticated slot per address taken by the app's own dial at that moment.
         */
        private suspend fun <T> retry(call: suspend () -> T): T {
            val deadline = SystemClock.elapsedRealtime() + 20_000
            while (true) {
                try { return call() }
                catch (error: zenpi.transport.WireException) {
                    val transient = error is HostRefusedException && error.code == "busy" || error.message?.startsWith("Host disconnected") == true
                    if (!transient || SystemClock.elapsedRealtime() > deadline) throw error
                }
                delay(100)
            }
        }

        suspend fun read(): HostProjection = retry { client().inspect(host, identity = identity, previous = last) }.also { last = it }

        /** A full snapshot, the reference for "no duplicate, no gap"; taken while the session is idle. */
        suspend fun snapshot(): HostProjection = retry { client().inspect(host, identity = identity) }

        /**
         * Sends [text] as a prompt, retried with the same request id (the host dedupes it) until the host
         * confirms it; a refused read after the confirmation is not retried, an unknown outcome fails.
         */
        suspend fun prompt(text: String) {
            val command = WorkspaceCommand.prompt(read(), text)
            var confirmed = false
            try {
                retry {
                    try { last = client().inspect(host, identity = identity, previous = last, workspaceCommand = command, onWorkspaceConfirmed = { confirmed = true }) }
                    catch (error: zenpi.transport.WireException) { if (!confirmed) throw error }
                }
            } catch (unknown: WorkspaceUnknownOutcomeException) { throw AssertionError("prompt '$text' outcome unknown", unknown) }
        }
    }

    private suspend fun <T : Any> until(timeoutMs: Long, what: String, probe: suspend () -> T?): T {
        val deadline = SystemClock.elapsedRealtime() + timeoutMs
        while (true) {
            probe()?.let { return it }
            check(SystemClock.elapsedRealtime() < deadline) { "timed out: $what" }
            delay(50)
        }
    }

    /**
     * Milliseconds from [since] until [probe] holds, polled every 10 ms. UiAutomator lookups query the
     * screen as it is: by default each first waits for 500 ms without accessibility events.
     */
    private fun elapsedUntil(since: Long, timeoutMs: Long, what: String, probe: () -> Boolean): Long {
        val idle = Configurator.getInstance().waitForIdleTimeout
        Configurator.getInstance().waitForIdleTimeout = 0
        try {
            while (!probe()) {
                check(SystemClock.elapsedRealtime() - since < timeoutMs) { "timed out: $what" }
                SystemClock.sleep(10)
            }
        } finally { Configurator.getInstance().waitForIdleTimeout = idle }
        return SystemClock.elapsedRealtime() - since
    }

    private fun head(projection: HostProjection?): RemoteApproval? =
        projection?.projects?.singleOrNull { it.id == projection.selectedId }?.pendingApprovals?.firstOrNull()

    private fun resumedMain(): MainActivity? {
        var activity: MainActivity? = null
        instrumentation.runOnMainSync {
            activity = ActivityLifecycleMonitorRegistry.getInstance().getActivitiesInStage(Stage.RESUMED).filterIsInstance<MainActivity>().firstOrNull()
        }
        return activity
    }

    /** MainActivity in the foreground, connected to the host; its view model. */
    private suspend fun foregroundApp(): PairingViewModel {
        if (resumedMain() == null) context.startActivity(Intent(context, MainActivity::class.java).addFlags(Intent.FLAG_ACTIVITY_NEW_TASK))
        val activity = until(10_000, "MainActivity resumed") { resumedMain() }
        var model: PairingViewModel? = null
        instrumentation.runOnMainSync { model = ViewModelProvider(activity)[PairingViewModel::class.java] }
        val state = requireNotNull(model).state
        try { until(15_000, "app connected") { state.value.takeIf { it.phase == PairingPhase.CONNECTED && it.projection != null } } }
        catch (timeout: IllegalStateException) { throw AssertionError("app not connected: ${state.value.phase} ${state.value.error}", timeout) }
        return requireNotNull(model)
    }

    @Test fun a_backgroundApprovalNotificationWithinTwoSeconds() = runBlocking<Unit> {
        val desktop = desktop()
        instrumentation.uiAutomation.grantRuntimePermission(context.packageName, android.Manifest.permission.POST_NOTIFICATIONS)
        // The app was in use (connected, its conversation cached), then left.
        foregroundApp()
        val cached = until(5_000, "conversation cached") { PairedHostStore(context).loadState()?.projection }
        device.pressHome()
        // The scripted provider answers this first prompt with a write_file call that waits for approval.
        desktop.prompt("ZE591 approval")
        // The background read (ResumeWorker, started now rather than 30 s after onStop). Delivered = in the shade.
        device.openNotification(); device.waitForIdle()
        ResumeScheduler.cancel(context)
        val started = SystemClock.elapsedRealtime()
        val work = ResumeScheduler.enqueue(context, delaySeconds = 0)
        val shown = try {
            elapsedUntil(started, 10_000, "notification shown") { device.hasObject(By.text(TITLE)) }
        } catch (timeout: IllegalStateException) {
            val info = androidx.work.WorkManager.getInstance(context).getWorkInfoById(work).get()
            // The same read as ResumeWorker's (without acknowledging), to name the refusal.
            val refusal = runCatching {
                client().inspect(desktop.host, cached.selectedId, desktop.identity, previous = cached, recoverSessionChanges = true)
            }.exceptionOrNull()
            device.pressBack()
            throw AssertionError("no approval notification within 10 s; ResumeWorker ${info?.state} after ${info?.runAttemptCount} " +
                "attempt(s); its host read: ${refusal ?: "succeeded"}", timeout)
        }
        Log.i("ZE591", "approval notification shown_ms=$shown (from the background read's start)")
        device.pressBack()
        assertTrue("approval notification shown after $shown ms", shown <= 2_000)
    }

    @Test fun b_notificationDeepLinkShowsTheSessionWithinOneSecond() = runBlocking<Unit> {
        val desktop = desktop()
        instrumentation.uiAutomation.grantRuntimePermission(context.packageName, android.Manifest.permission.POST_NOTIFICATIONS)
        // The pending approval as the app last saw it (its subscription keeps the queue while the turn runs).
        val state = foregroundApp().state
        if (head(state.value.projection) == null) desktop.prompt("ZE591 approval")
        val projection = until(20_000, "approval pending in the app") { state.value.projection?.takeIf { head(it) != null } }
        try {
            // Three taps from the background, each on the notification ResumeWorker posts for this
            // approval (a_ covers whether it gets to post it).
            val opened = (1..3).map {
                device.pressHome()
                ResumeScheduler.cancel(context)
                device.openNotification(); device.waitForIdle()
                val posting = SystemClock.elapsedRealtime()
                assertTrue(ApprovalNotifier.sync(context, "v1", projection))
                val inShade = elapsedUntil(posting, 5_000, "notification in the shade") { device.hasObject(By.text(TITLE)) }
                Log.i("ZE591", "ApprovalNotifier.sync to the notification in the shade ms=$inShade")
                val entry = requireNotNull(device.findObject(By.text(TITLE)))
                val tapped = SystemClock.elapsedRealtime()
                entry.click()
                val resumed = elapsedUntil(tapped, 10_000, "app in front") { resumedMain() != null }
                val shown = elapsedUntil(tapped, 10_000, "session with its approval bar") { device.hasObject(By.desc("Allow approval")) }
                Log.i("ZE591", "deep link: MainActivity resumed ms=$resumed, session with its approval bar ms=$shown " +
                    "(the approval's prompt turn shown: ${device.hasObject(By.text("ZE591 approval"))})")
                val now = foregroundApp().state.value.projection
                assertEquals(projection.selectedId, now?.selectedId); assertEquals(projection.sessionId, now?.sessionId)
                shown
            }
            assertTrue("deep link reached the session after $opened ms", opened.all { it <= 1_000 })
        } finally { settle(desktop) }
    }

    /** Allows any pending approval from the app's bar and waits for the run to finish. */
    private suspend fun settle(desktop: Desktop) {
        val state = foregroundApp().state
        if (head(state.value.projection) == null) return
        requireNotNull(device.findObject(By.desc("Allow approval"))).click()
        until(20_000, "approval left the queue") { state.value.projection?.takeIf { head(it) == null } }
        val settled = until(20_000, "approved run finished") { desktop.read().takeIf { it.phase != "running" } }
        assertTrue(settled.turns.any { it.content.contains("ZE591_REPLY") })
    }

    @Test fun c_reconnectAfterThirtySecondsOfflineHasNoDuplicateOrMissingTurnAndKeepsTheDraft() = runBlocking<Unit> {
        val desktop = desktop()
        settle(desktop)
        val model = foregroundApp()
        val draft = "ZE591 draft typed before going offline"
        val composer = By.clazz("android.widget.EditText")
        requireNotNull(device.findObject(composer)).text = draft
        try { until(5_000, "draft in the composer") { device.findObject(composer)?.text?.takeIf { it == draft } } }
        catch (timeout: IllegalStateException) { throw AssertionError("composer shows '${device.findObject(composer)?.text}'", timeout) }
        val before = requireNotNull(model.state.value.projection).turns
        try {
            val offline = SystemClock.elapsedRealtime()
            Network.offline(true)
            until(10_000, "app offline") { model.state.value.takeIf { it.phase == PairingPhase.OFFLINE } }
            // The desktop keeps working meanwhile: three prompts, each answered.
            for (index in 1..3) {
                desktop.prompt("ZE591 offline $index")
                until(20_000, "offline prompt $index answered") {
                    desktop.read().takeIf { live -> live.phase != "running" && live.turns.any { it.content == "ZE591 offline $index" } }
                }
            }
            delay(maxOf(0L, 30_000 - (SystemClock.elapsedRealtime() - offline)))
            val offlineMs = SystemClock.elapsedRealtime() - offline
            Network.offline(false)
            val online = SystemClock.elapsedRealtime()  // Wi-Fi is back
            val hostTurns = desktop.snapshot().turns
            val turns = until(30_000, "app caught up with the host") {
                model.state.value.takeIf { it.phase == PairingPhase.CONNECTED }?.projection?.turns
                    ?.takeIf { it.map { turn -> turn.id } == hostTurns.map { turn -> turn.id } }
            }
            val caughtUp = SystemClock.elapsedRealtime() - online
            val ids = turns.map { it.id }
            Log.i("ZE591", "offline_ms=$offlineMs reconnect_caught_up_ms=$caughtUp turns_before=${before.size} turns_after=${turns.size} host_turns=${hostTurns.size}")
            assertTrue("offline for $offlineMs ms", offlineMs >= 30_000)
            assertEquals("no duplicate turn", ids.size, ids.toSet().size)
            assertEquals("no missing turn: the app holds exactly the host's turns", hostTurns, turns)
            assertTrue("the turns seen before going offline are kept", ids.containsAll(before.map { it.id }))
            for (index in 1..3) assertEquals("offline prompt $index once", 1, turns.count { it.content == "ZE591 offline $index" })
            // On screen too: the newest offline prompt is rendered once, and the draft survived.
            until(5_000, "newest offline prompt on screen") { device.findObject(By.text("ZE591 offline 3")) }
            assertEquals(1, device.findObjects(By.text("ZE591 offline 3")).size)
            assertEquals("the draft survived", draft, until(5_000, "composer on screen") { device.findObject(composer) }.text)
        } finally { Network.offline(false) }
    }
}

/**
 * Airplane mode through the shell (the emulator's adb transport and adb reverse do not use the guest
 * network). Back online means Wi-Fi again: the app dials only over Wi-Fi/Ethernet, and the emulator's
 * virtual access point is rejoined explicitly.
 */
private object Network {
    private fun shell(command: String) = InstrumentationRegistry.getInstrumentation().uiAutomation.executeShellCommand(command).close()

    fun offline(on: Boolean) {
        val connectivity = InstrumentationRegistry.getInstrumentation().targetContext.getSystemService(android.net.ConnectivityManager::class.java)
        fun wifi() = connectivity.activeNetwork?.let(connectivity::getNetworkCapabilities)
            ?.hasTransport(android.net.NetworkCapabilities.TRANSPORT_WIFI) == true
        shell("cmd connectivity airplane-mode ${if (on) "enable" else "disable"}")
        val deadline = SystemClock.elapsedRealtime() + 30_000
        var joined = SystemClock.elapsedRealtime()
        while (if (on) connectivity.activeNetwork != null else !wifi()) {
            check(SystemClock.elapsedRealtime() < deadline) { "airplane mode $on not reached" }
            if (!on && SystemClock.elapsedRealtime() >= joined) { shell("cmd wifi connect-network AndroidWifi open"); joined += 5_000 }
            SystemClock.sleep(50)
        }
    }
}
