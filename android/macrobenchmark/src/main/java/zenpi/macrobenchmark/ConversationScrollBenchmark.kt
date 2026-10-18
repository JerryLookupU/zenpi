package zenpi.macrobenchmark

import android.annotation.SuppressLint
import android.content.Intent
import android.hardware.display.DisplayManager
import android.util.Log
import android.view.Display
import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.FrameTimingMetric
import androidx.benchmark.macro.macrobenchmarkWithStartupMode
import androidx.benchmark.perfetto.ExperimentalPerfettoCaptureApi
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Until
import org.junit.Assert.assertTrue
import org.junit.Test
import org.junit.runner.RunWith

/**
 * ZE-591: scrolling a 10,000-turn conversation (production AppShell and theme, R8 benchmark build,
 * JIT only) by real flings keeps the 95th-percentile frame at or above 55 fps: the P95 frame overrun
 * leaves the frame interval within 1000/55 ms, and the P95 frame CPU time fits in it too.
 */
@RunWith(AndroidJUnit4::class)
class ConversationScrollBenchmark {
    private val target = "app.zenpi.android"
    private companion object { const val TURNS = 10_000 }

    // MacrobenchmarkRule.measureRepeated returns nothing; this is the same entry point (it also
    // writes the usual JSON and IDE summary) and returns the merged metrics to assert on.
    @SuppressLint("RestrictedApi")
    @OptIn(ExperimentalPerfettoCaptureApi::class)
    @Test fun tenThousandTurnConversationScrollsAtP95AtLeast55Fps() {
        val result = macrobenchmarkWithStartupMode(
            uniqueName = "ConversationScrollBenchmark_tenThousandTurns", className = javaClass.name,
            testName = "tenThousandTurnConversationScrollsAtP95AtLeast55Fps", packageName = target,
            metrics = listOf(FrameTimingMetric()),
            // A sideloaded APK has no cloud or baseline profile: the first runs are JIT only.
            compilationMode = CompilationMode.None(), iterations = 5, perfettoConfig = null,
            // A fresh process per iteration; StartupMode.COLD would kill it again after this block.
            startupMode = null,
            setupBlock = {
                pressHome(); killProcess()
                startActivityAndWait(Intent().setClassName(target, "zenpi.benchmark.ConversationFixtureActivity"))
                check(device.wait(Until.hasObject(By.text("Turn ${TURNS - 1} found:")), 10_000)) { "newest turn not shown" }
            },
        ) {
            // Inside the conversation list (below the top bar, above the approval bar and composer).
            val (x, top, bottom) = Triple(device.displayWidth / 2, device.displayHeight * 35 / 100, device.displayHeight * 70 / 100)
            // Reverse layout: older turns are above. Fling back through the history, then return
            // (a 30-step swipe, about 150 ms, ends in a fling).
            for (older in listOf(true, false)) repeat(8) {
                device.swipe(x, if (older) top else bottom, x, if (older) bottom else top, 30)
                device.waitForIdle()
            }
        }
        val period = 1000.0 / InstrumentationRegistry.getInstrumentation().context.getSystemService(DisplayManager::class.java)
            .getDisplay(Display.DEFAULT_DISPLAY).refreshRate
        val cpu = result.sampledMetrics.getValue("frameDurationCpuMs")
        val overrun = result.sampledMetrics.getValue("frameOverrunMs")
        // A frame overrunning its deadline by d ms is shown one refresh period plus d after the last one.
        val p95Fps = 1000.0 / (period + maxOf(0.0, overrun.P95))
        fun ms(value: Double) = "%.2f".format(value)
        val report = "frames=${cpu.runs.sumOf { it.size }} refresh_ms=${ms(period)} frameDurationCpuMs P50=${ms(cpu.P50)} " +
            "P90=${ms(cpu.P90)} P95=${ms(cpu.P95)} P99=${ms(cpu.P99)} frameOverrunMs P50=${ms(overrun.P50)} P90=${ms(overrun.P90)} " +
            "P95=${ms(overrun.P95)} P99=${ms(overrun.P99)} p95_fps=${"%.1f".format(p95Fps)}"
        Log.i("ZE591", report)
        assertTrue(report, cpu.runs.sumOf { it.size } >= 100)
        assertTrue(report, p95Fps >= 55 && cpu.P95 <= 1000.0 / 55)
    }
}
