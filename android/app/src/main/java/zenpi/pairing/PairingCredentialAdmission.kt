package zenpi.pairing

import kotlinx.coroutines.CancellationException
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.ensureActive
import kotlinx.coroutines.withContext
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock

/** Serialize non-suspending Keystore creation with save/reset, including retired jobs. */
internal object PairingCredentialAdmission {
    suspend fun <T> load(storage: Mutex, isCurrent: () -> Boolean, requireFresh: () -> Unit = {}, load: () -> T): T = storage.withLock {
        withContext(Dispatchers.IO) {
            ensureActive()
            if (!isCurrent()) throw CancellationException("Pairing replaced before key admission")
            requireFresh()
            load()
        }
    }
}
