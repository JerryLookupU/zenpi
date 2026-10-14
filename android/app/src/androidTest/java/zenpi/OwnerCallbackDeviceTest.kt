package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.ownerCallback

class OwnerCallbackDeviceTest {
    @Test fun networkCallbackRunsOnActualAndroidMainOwnerAndRejectsReplacedGeneration() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        var generation = 1
        var updates = 0
        val current = withContext(Dispatchers.IO) {
            ownerCallback(isCurrent = { generation == 1 }) {
                assertSame(android.os.Looper.getMainLooper(), android.os.Looper.myLooper())
                updates++
            }
        }
        assertEquals(Unit, current); assertEquals(1, updates)
        instrumentation.runOnMainSync { generation = 2 }
        val error = runCatching {
            withContext(Dispatchers.IO) {
                ownerCallback(isCurrent = { generation == 1 }) { updates++ }
            }
        }.exceptionOrNull()
        assertTrue(error is CancellationException); assertEquals(1, updates)
    }
}
