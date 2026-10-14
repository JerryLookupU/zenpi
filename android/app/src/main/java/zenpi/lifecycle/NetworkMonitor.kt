package zenpi.lifecycle

import android.content.Context
import android.net.ConnectivityManager
import android.net.Network
import android.net.NetworkCapabilities
import android.net.NetworkRequest
import android.net.LinkProperties
import android.os.Handler
import android.os.Looper

/** Default plus local LAN inventory; the token fences route changes, not Internet validation. */
class NetworkMonitor(context: Context, private val changed: (Long?) -> Unit) {
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)
    private val main = Handler(Looper.getMainLooper())
    private var callback: ConnectivityManager.NetworkCallback? = null
    private var localCallback: ConnectivityManager.NetworkCallback? = null
    private var reconciliation: Runnable? = null
    private var generation = 0
    private var lastPublished: Long? = null
    fun start() {
        check(Looper.myLooper() == Looper.getMainLooper())
        if (callback != null) return
        val current = ++generation
        val route = NetworkInventoryState<NetworkLinkStamp>()
        fun refresh(initial: Boolean = false) {
            if (generation != current) return
            val observation = NetworkObservation.read(onFailure = {
                // Retire callbacks before publishing offline; queued events from
                // this cycle must not revive a route after an observation failed.
                stop()
                lastPublished = null
                changed(null)
            }) {
                val locals = connectivity.allNetworks.mapNotNull { network ->
                    val caps = connectivity.getNetworkCapabilities(network)
                    val links = connectivity.getLinkProperties(network)
                    // A VPN network counts only as a possible T1 overlay route; the
                    // socket binder still admits it for overlay targets alone.
                    if (caps != null && links != null
                        && caps.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
                        && (caps.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
                            || caps.hasTransport(NetworkCapabilities.TRANSPORT_WIFI) || caps.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET)))
                        network.networkHandle to NetworkLinkStamp(links.interfaceName, links.mtu,
                            links.linkAddresses.map { LanAddressState(it.address.hostAddress, it.prefixLength, it.flags, it.scope) }.toSet(),
                            links.routes.map { NetworkRouteStamp(it.destination.address.hostAddress, it.destination.prefixLength,
                                it.gateway?.hostAddress, it.typeCompat, it.`interface`) }.toSet()) else null
                }.toMap()
                connectivity.activeNetwork?.networkHandle to locals
            } ?: return
            if (route.update(observation.first, observation.second, force = initial)
                && (initial || route.current != lastPublished)) {
                lastPublished = route.current
                changed(route.current)
            }
        }
        val listener = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) { main.post { refresh() } }
            override fun onLost(network: Network) { main.post { refresh() } }
            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) { main.post { refresh() } }
            override fun onCapabilitiesChanged(network: Network, networkCapabilities: NetworkCapabilities) { main.post { refresh() } }
        }
        val local = object : ConnectivityManager.NetworkCallback() {
            override fun onAvailable(network: Network) { main.post { refresh() } }
            override fun onLost(network: Network) { main.post { refresh() } }
            override fun onLinkPropertiesChanged(network: Network, linkProperties: LinkProperties) { main.post { refresh() } }
            override fun onCapabilitiesChanged(network: Network, networkCapabilities: NetworkCapabilities) { main.post { refresh() } }
        }
        // Publish synchronously before onStart can initiate network work.
        try {
            refresh(initial = true)
            if (generation != current) return
            connectivity.registerDefaultNetworkCallback(listener); callback = listener
            connectivity.registerNetworkCallback(NetworkRequest.Builder().clearCapabilities()
                .addCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
                .addTransportType(NetworkCapabilities.TRANSPORT_WIFI)
                .addTransportType(NetworkCapabilities.TRANSPORT_ETHERNET)
                .addTransportType(NetworkCapabilities.TRANSPORT_VPN).build(), local)
            localCallback = local
            refresh()
            // Callback registration/loss can race the inventory snapshot. While
            // foreground-owned, reconcile local metadata too; never keep a socket
            // alive or manufacture a reachable LAN from a default route.
            if (generation == current) {
                val poll = object : Runnable {
                    override fun run() {
                        if (generation != current) return
                        refresh()
                        if (generation == current) main.postDelayed(this, 1_000)
                    }
                }
                reconciliation = poll
                main.postDelayed(poll, 1_000)
            }
        } catch (_: RuntimeException) { stop(); lastPublished = null; changed(null) }
    }
    fun stop() {
        check(Looper.myLooper() == Looper.getMainLooper())
        generation++
        val default = callback
        val local = localCallback
        callback = null
        localCallback = null
        reconciliation?.let(main::removeCallbacks)
        reconciliation = null
        // Android may report a callback as already unregistered after a
        // process/network transition. Teardown must remain idempotent and may
        // not crash Activity.onStop or hide the offline transition.
        try { default?.let { runCatching { connectivity.unregisterNetworkCallback(it) } } }
        finally { local?.let { runCatching { connectivity.unregisterNetworkCallback(it) } } }
    }
}

/** Android network inventory failures invalidate the observation, not the app. */
internal object NetworkObservation {
    fun <T : Any> read(onFailure: () -> Unit, query: () -> T): T? {
        val result = try { query() }
        catch (_: RuntimeException) { onFailure(); return null }
        return result
    }
}

private data class NetworkRouteStamp(val destination: String?, val prefix: Int, val gateway: String?, val type: Int, val networkInterface: String?)
private data class NetworkLinkStamp(val networkInterface: String?, val mtu: Int,
    val addresses: Set<LanAddressState>, val routes: Set<NetworkRouteStamp>)

internal class NetworkInventoryState<T> {
    companion object { private val revisions = java.util.concurrent.atomic.AtomicLong() }
    private var inventory: Pair<Long?, Map<Long, T>>? = null
    var current: Long? = null
        private set
    fun update(default: Long?, locals: Map<Long, T>, force: Boolean = false): Boolean {
        val next = default to locals.toMap()
        if (!force && inventory == next) return false
        inventory = next
            // Pairing is LAN-first: a cellular/default route is not enough to
            // claim that the saved host is reachable.  The socket binder will
            // still select the exact matching Wi-Fi/Ethernet route per host;
            // this token only prevents the UI from retrying while no LAN
            // network exists at all.
            current = if (locals.isEmpty()) null else revisions.updateAndGet {
            check(it < Long.MAX_VALUE) { "Network observation revision exhausted" }
            it + 1
        }
        return true
    }
}

internal class DefaultRouteState {
    var current: Long? = null
        private set
    fun available(route: Long?): Boolean {
        val changed = current != route
        current = route
        return changed
    }
    fun lost(route: Long?): Boolean {
        if (current != route || current == null) return false
        current = null
        return true
    }
}
