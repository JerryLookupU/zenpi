package zenpi.lifecycle

import android.content.Context
import android.net.ConnectivityManager
import android.net.NetworkCapabilities
import android.net.RouteInfo
import android.os.Build
import java.io.IOException
import java.net.InetAddress
import java.net.Socket

/** RouteInfo#getType is API 33+; before that the public route list carries no type, so treat it as unicast. */
internal val RouteInfo.typeCompat: Int
    get() = if (Build.VERSION.SDK_INT >= Build.VERSION_CODES.TIRAMISU) type else RouteInfo.RTN_UNICAST

internal data class LanRoutePath(val destination: String?, val gateway: String?, val networkInterface: String?)
internal data class LanAddressState(val address: String?, val prefixLength: Int, val flags: Int, val scope: Int)
internal data class LanLinkState(val networkInterface: String?, val mtu: Int,
    val addresses: Set<LanAddressState>)
internal data class LanRoute(val network: Long, val prefixLength: Int, val forwarding: Boolean = true,
    val path: LanRoutePath? = null, val link: LanLinkState? = null)

internal object LanRoutePolicy {
    private fun effective(address: InetAddress): InetAddress {
        val bytes = address.address
        return if (bytes.size == 16 && bytes.take(10).all { it == 0.toByte() }
            && bytes[10] == 0xff.toByte() && bytes[11] == 0xff.toByte()) {
            InetAddress.getByAddress(bytes.copyOfRange(12, 16))
        } else address
    }

    fun literal(host: String): Boolean {
        val valid = if (host.contains(':')) {
            // Zone-scoped IPv6 literals are not stable across Android network
            // handles and cannot be passed safely through the QR URI.
            if (host.contains('%')) return false
            if (!host.matches(Regex("[a-fA-F0-9:.]+"))) return false
            // The character whitelist alone accepts malformed forms such as
            // "::::". Validate the literal without permitting DNS fallback.
            runCatching { InetAddress.getByName(host) }.isSuccess
        }
        else {
            val octets = host.split('.')
            octets.size == 4 && octets.all { it.matches(Regex("0|[1-9][0-9]{0,2}")) && it.toInt() <= 255 }
        }
        if (!valid) return false
        val address = runCatching { effective(InetAddress.getByName(host)) }.getOrNull() ?: return false
        // A saved or tampered paired-host record must not redirect the socket
        // to an unspecified, multicast, link-local, or broadcast destination.
        return !address.isAnyLocalAddress && !address.isMulticastAddress
            && !address.isLinkLocalAddress && address.hostAddress != "255.255.255.255"
    }
    fun select(routes: List<LanRoute>): Long? {
        if (routes.any { it.prefixLength !in 1..128 }) throw IOException("Invalid LAN route prefix")
        val longest = routes.maxOfOrNull { it.prefixLength } ?: return null
        val matching = routes.filter { it.prefixLength == longest }
        if (matching.any { !it.forwarding }) throw IOException("LAN route does not forward")
        val candidates = matching.map { it.network }.distinct()
        if (candidates.size != 1) throw IOException("Ambiguous LAN route")
        return candidates.single()
    }

    fun bindStable(observe: () -> List<LanRoute>, bind: (Long) -> Unit) {
        val before = observe().toSet()
        val selected = select(before.toList()) ?: throw IOException("Network unavailable")
        bind(selected)
        // Binding does not freeze LinkProperties. Re-read before callers can
        // connect and refuse route changes observed across the bind operation.
        val after = observe().toSet()
        if (select(after.toList()) != selected || after != before)
            throw IOException("LAN route changed during socket binding")
    }
}

/** Bind only this unconnected socket, never the process or system default route. */
class LanSocketRouting(context: Context) {
    private val connectivity = context.getSystemService(ConnectivityManager::class.java)

    private fun isLanNetwork(network: android.net.Network): Boolean {
        val capabilities = connectivity.getNetworkCapabilities(network) ?: return false
        return !capabilities.hasTransport(NetworkCapabilities.TRANSPORT_VPN)
            && capabilities.hasCapability(NetworkCapabilities.NET_CAPABILITY_NOT_RESTRICTED)
            && (capabilities.hasTransport(NetworkCapabilities.TRANSPORT_WIFI)
                || capabilities.hasTransport(NetworkCapabilities.TRANSPORT_ETHERNET))
    }

    fun bind(socket: Socket, host: String) {
        if (socket.isConnected || socket.isClosed) throw IOException("LAN socket must be unconnected")
        if (!LanRoutePolicy.literal(host)) throw IOException("Literal host required")
        val address = try { InetAddress.getByName(host) }
        catch (_: Exception) { throw IOException("Invalid literal host", null) }
        // Loopback fixtures use adb reverse rather than a physical interface.
        if (address.isLoopbackAddress) return
        // A missing matching route is not proof that the target is reachable:
        // falling back to the active network would silently turn a LAN-only
        // pairing into a default-route/cellular/VPN attempt.  Require the
        // network whose non-default route matched the literal target.
        LanRoutePolicy.bindStable(observe = {
            connectivity.allNetworks.flatMap { network ->
                if (!isLanNetwork(network)) emptyList()
                else {
                    val properties = connectivity.getLinkProperties(network)
                    val link = properties?.let { LanLinkState(it.interfaceName, it.mtu,
                        it.linkAddresses.map { entry -> LanAddressState(entry.address.hostAddress, entry.prefixLength,
                            entry.flags, entry.scope) }.toSet()) }
                    properties?.routes.orEmpty().filter {
                        !it.isDefaultRoute && it.matches(address)
                    }.map { LanRoute(network.networkHandle, it.destination.prefixLength, it.typeCompat == RouteInfo.RTN_UNICAST,
                        LanRoutePath(it.destination.address.hostAddress, it.gateway?.hostAddress, it.`interface`), link) }
                }
            }
        }, bind = { handle ->
            val network = connectivity.allNetworks.singleOrNull { it.networkHandle == handle }?.takeIf(::isLanNetwork)
            (network ?: throw IOException("Network unavailable")).bindSocket(socket)
        })
    }
}
