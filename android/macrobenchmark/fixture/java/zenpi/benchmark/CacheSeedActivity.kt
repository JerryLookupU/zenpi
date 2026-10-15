package zenpi.benchmark

import android.app.Activity
import android.os.Bundle
import zenpi.security.PairedHost
import zenpi.security.PairedHostStore
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemoteTurn

/**
 * ZE-591 Macrobenchmark fixture (benchmark build type only): writes MainActivity's own encrypted
 * store, a paired host and its cached 64-turn conversation (the display tail), as a previous session
 * would have left it; `wipe=true` removes it again.
 */
class CacheSeedActivity : Activity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        val store = PairedHostStore(this)
        store.wipe()
        if (!intent.getBooleanExtra("wipe", false)) {
            val host = PairedHost("192.168.1.20", 7443, "e".repeat(64), "d".repeat(64))
            store.save(host)
            store.saveProjection(host, HostProjection(listOf(RemoteProject("p", "zenpi", "s")), "p", "s", "idle",
                (0 until 64).map { RemoteTurn("t$it", if (it % 2 == 0) "user" else "assistant", "ZE591_CACHED_$it: the cached reply") },
                "64", false, "64"))
        }
        finish()
    }
}
