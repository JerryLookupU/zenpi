package zenpi

import android.os.Handler
import android.os.HandlerThread
import android.os.SystemClock
import android.view.FrameMetrics
import android.view.MotionEvent
import android.view.Window
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.compose.material3.MaterialTheme
import androidx.test.core.app.ActivityScenario
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.assertTrue
import org.junit.Test
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.security.PairedHost
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteToolCall
import zenpi.transport.RemoteTurn
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/**
 * ZE-530 validator: a 500-turn conversation (markdown, code blocks, collapsed tool rows and a
 * streaming row) scrolled by real touch drags and flings in both directions renders fewer than
 * 5% janky frames. A frame is janky when it takes more than twice the display refresh period,
 * the default heuristic of androidx JankStats (measured here with the platform FrameMetrics API).
 */
class ConversationJankTest {
    private fun turn(index: Int): RemoteTurn = if (index % 2 == 0) RemoteTurn("t$index", "user", "Step $index: check `module_$index` and report back.")
    else RemoteTurn("t$index", "assistant", "Turn $index found:\n- `src/file_$index.rs` changed\n- tests ${index % 7} pending\n" +
        "```rust\nfn check_$index(value: u32) -> bool { value % $index == 0 } // a long line that scrolls sideways\n```\nDone.",
        toolCalls = listOf(RemoteToolCall("c$index", "bash", "{\"command\":\"cargo test case_$index\"}", false)))

    private val state = PairingState(phase = PairingPhase.CONNECTED,
        host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64)), registered = true,
        projection = HostProjection(listOf(RemoteProject("p", "zenpi", "s", workspaceControl = true, promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("r", "p", "s", "running", streamText = "Streaming `partial` answer…")))),
            "p", "s", "running", (0 until 500).map(::turn), "500", false, "500", workspaceOwner = "f".repeat(64), workspaceRevision = "1"))

    @Test fun fiveHundredTurnConversationScrollsWithUnderFivePercentJankyFrames() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val frames = java.util.Collections.synchronizedList(mutableListOf<Long>())
        val thread = HandlerThread("frame-metrics").apply { start() }
        val listener = Window.OnFrameMetricsAvailableListener { _, metrics, _ ->
            if (metrics.getMetric(FrameMetrics.FIRST_DRAW_FRAME) == 0L) frames += metrics.getMetric(FrameMetrics.TOTAL_DURATION)
        }
        var refresh = 0L; var width = 0; var height = 0
        ActivityScenario.launch(ComponentActivity::class.java).use { scenario ->
            scenario.onActivity { activity ->
                activity.setContent { MaterialTheme { AppShell(state, object : PairingActions {}) } }
                refresh = (1_000_000_000 / activity.display.refreshRate).toLong()
                width = activity.window.decorView.width; height = activity.window.decorView.height
            }
            instrumentation.waitForIdleSync(); SystemClock.sleep(1500)
            scenario.onActivity { it.window.addOnFrameMetricsAvailableListener(listener, Handler(thread.looper)) }
            // Older turns are above in the reverse list: drag down to read back, then up to return.
            for (down in listOf(true, false)) repeat(12) {
                drag(width / 2f, if (down) height * 0.3f else height * 0.7f, if (down) height * 0.7f else height * 0.3f)
                SystemClock.sleep(600)
            }
            scenario.onActivity { it.window.removeOnFrameMetricsAvailableListener(listener) }
        }
        thread.quitSafely()
        val janky = frames.count { it > 2 * refresh }
        val sorted = frames.sorted()
        fun percentile(p: Int) = sorted.getOrNull(sorted.size * p / 100)?.div(1000)
        val report = "frames=${frames.size} janky=$janky (${"%.2f".format(100.0 * janky / frames.size.coerceAtLeast(1))}%) " +
            "refresh=${refresh / 1000}us p50=${percentile(50)}us p90=${percentile(90)}us p95=${percentile(95)}us p99=${percentile(99)}us"
        android.util.Log.i("ConversationJank", report)
        assertTrue(report, frames.size >= 100)
        assertTrue(report, janky * 100 < frames.size * 5)
    }

    /** A real-time touch drag ending in a fling, 16 ms between moves. */
    private fun drag(x: Float, fromY: Float, toY: Float) {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val start = SystemClock.uptimeMillis()
        fun send(action: Int, y: Float, time: Long) = MotionEvent.obtain(start, time, action, x, y, 0).also {
            it.source = android.view.InputDevice.SOURCE_TOUCHSCREEN
            instrumentation.uiAutomation.injectInputEvent(it, true); it.recycle()
        }
        send(MotionEvent.ACTION_DOWN, fromY, start)
        val steps = 10
        for (step in 1..steps) {
            SystemClock.sleep(16)
            send(MotionEvent.ACTION_MOVE, fromY + (toY - fromY) * step / steps, SystemClock.uptimeMillis())
        }
        send(MotionEvent.ACTION_UP, toY, SystemClock.uptimeMillis())
    }
}
