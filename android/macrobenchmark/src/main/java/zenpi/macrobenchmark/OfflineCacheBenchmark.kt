package zenpi.macrobenchmark

import android.net.ConnectivityManager
import android.os.SystemClock
import android.util.Log
import androidx.benchmark.macro.CompilationMode
import androidx.benchmark.macro.StartupMode
import androidx.benchmark.macro.StartupTimingMetric
import androidx.benchmark.macro.junit4.MacrobenchmarkRule
import androidx.test.ext.junit.runners.AndroidJUnit4
import androidx.test.platform.app.InstrumentationRegistry
import androidx.test.uiautomator.By
import androidx.test.uiautomator.Configurator
import androidx.test.uiautomator.UiDevice
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith

/**
 * ZE-591: offline, opening the app renders the cached conversation within 500 ms. The R8 benchmark
 * build's own MainActivity reads the encrypted store seeded by CacheSeedActivity, in airplane mode;
 * the time is from the launch intent of a cold process (JIT only) until the newest cached turn is on
 * screen (accessibility).
 */
@RunWith(AndroidJUnit4::class)
class OfflineCacheBenchmark {
    @get:Rule val rule = MacrobenchmarkRule()
    private val target = "app.zenpi.android"
    private val instrumentation = InstrumentationRegistry.getInstrumentation()
    private val device = UiDevice.getInstance(instrumentation)

    private fun offline(on: Boolean) {
        device.executeShellCommand("cmd connectivity airplane-mode ${if (on) "enable" else "disable"}")
        val connectivity = instrumentation.context.getSystemService(ConnectivityManager::class.java)
        val deadline = SystemClock.elapsedRealtime() + 15_000
        while ((connectivity.activeNetwork == null) != on) {
            check(SystemClock.elapsedRealtime() < deadline) { "airplane mode $on not reached" }
            SystemClock.sleep(50)
        }
    }

    @Test fun offlineColdLaunchRendersTheCachedConversationWithin500Ms() {
        val rendered = mutableListOf<Long>()
        // Query the screen as it is: by default every UiAutomator lookup first waits for 500 ms
        // without accessibility events, which would be counted as render time.
        val idle = Configurator.getInstance().waitForIdleTimeout
        Configurator.getInstance().waitForIdleTimeout = 0
        device.executeShellCommand("am start -W -n $target/zenpi.benchmark.CacheSeedActivity")
        offline(true)
        try {
            rule.measureRepeated(packageName = target, metrics = listOf(StartupTimingMetric()),
                compilationMode = CompilationMode.None(), startupMode = StartupMode.COLD, iterations = 5,
                setupBlock = { pressHome() }) {
                val launched = SystemClock.elapsedRealtime()
                device.executeShellCommand("am start -n $target/zenpi.MainActivity")
                while (!device.hasObject(By.textContains("ZE591_CACHED_63"))) {
                    check(SystemClock.elapsedRealtime() - launched < 10_000) { "cached conversation not shown" }
                    SystemClock.sleep(5)
                }
                rendered += SystemClock.elapsedRealtime() - launched
                check(device.hasObject(By.textContains("Network unavailable"))) { "not the offline path" }
            }
        } finally {
            Configurator.getInstance().waitForIdleTimeout = idle
            offline(false)
            device.executeShellCommand("am start -W -n $target/zenpi.benchmark.CacheSeedActivity --ez wipe true")
        }
        Log.i("ZE591", "offline cold launch, cached conversation on screen ms=$rendered")
        assertTrue("cold offline launch rendered the cache after $rendered ms", rendered.all { it <= 500 })
    }
}
