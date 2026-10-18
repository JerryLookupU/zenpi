package zenpi.pairing

import java.net.InetAddress
import java.net.URI
import java.net.URLDecoder
import java.util.Base64

class PairingException(val reason: String) : Exception(reason)

/** One ordered dial hint from the QR; routing advice only, identity is the pinned key. */
data class AddressHint(val kind: String, val host: String, val port: Int) {
    val address: String get() = if (host.contains(':')) "[$host]:$port" else "$host:$port"
    companion object { val KINDS = setOf("lan", "tailnet") }
}

/**
 * v1: `address` + `fingerprint` (certificate pin). v2: `host_id` (`sha256(SPKI)`) +
 * ordered `hints`. [fingerprint] is the pin either way; [version] tells which.
 */
class PairingInvite private constructor(
    val version: Int, val hints: List<AddressHint>, val fingerprint: String,
    internal val token: String, val expiresAt: Long,
    private val admittedAtNanos: Long, private val lifetimeNanos: Long, private val monotonicNow: () -> Long,
) {
    private var expired = false
    val host: String get() = hints.first().host
    val port: Int get() = hints.first().port
    val address: String get() = hints.first().address
    @Synchronized fun requireFresh(now: Long = System.currentTimeMillis() / 1000) {
        freshElapsed(now)
    }
    private fun freshElapsed(now: Long): Long {
        // Avoid subtraction overflow when this boundary is exercised with a
        // corrupted clock value; the host's u64 expiry window is at most 300s.
        val latest = if (now > Long.MAX_VALUE - 300) Long.MAX_VALUE else now + 300
        val elapsed = monotonicNow() - admittedAtNanos
        if (expired || now < 0 || expiresAt <= now || expiresAt > latest || elapsed < 0 || elapsed >= lifetimeNanos) {
            expired = true
            throw PairingException("Invitation expired or invalid")
        }
        return elapsed
    }
    @Synchronized fun remainingSeconds(now: Long = System.currentTimeMillis() / 1000): Long {
        // Validate and render one observation; a second read can disagree at expiry.
        val elapsed = runCatching { freshElapsed(now) }.getOrElse { return 0 }
        val left = lifetimeNanos - elapsed
        return minOf(expiresAt - now, (left + 999_999_999) / 1_000_000_000)
    }
    override fun toString(): String = "PairingInvite(address=$address, token=<redacted>)"

    companion object {
        fun parse(text: String, now: Long = System.currentTimeMillis() / 1000,
            monotonicNow: () -> Long = { android.os.SystemClock.elapsedRealtimeNanos() }): PairingInvite {
            try {
                val admittedAt = monotonicNow()
                if (text.length > 2048 || text.toByteArray(Charsets.UTF_8).size > 2048 || text.any { it.code < 32 || it.code == 127 })
                    throw PairingException("Invalid invitation")
                val uri = URI(text)
                if (uri.scheme != "zenpi" || uri.rawAuthority != "pair" || !uri.rawPath.isNullOrEmpty()
                    || uri.rawFragment != null) throw PairingException("Invalid invitation")
                val fields = linkedMapOf<String, String>()
                for (part in (uri.rawQuery ?: "").split('&')) {
                    val pair = part.split('=', limit = 2)
                    if (pair.size != 2) throw PairingException("Invalid invitation")
                    // QR values are URI percent-encoded, not form data:
                    // '+' is a literal token/address character and must not
                    // silently become a space during decoding.
                    val key = java.net.URLDecoder.decode(pair[0].replace("+", "%2B"), "UTF-8")
                    val value = java.net.URLDecoder.decode(pair[1].replace("+", "%2B"), "UTF-8")
                    if (key !in setOf("v", "address", "fingerprint", "host_id", "hints", "token", "expires") || fields.put(key, value) != null)
                        throw PairingException("Invalid invitation")
                }
                // Each version has exactly its own field set.
                val version = when {
                    fields.keys == setOf("v", "address", "fingerprint", "token", "expires") && fields["v"] == "1" -> 1
                    fields.keys == setOf("v", "host_id", "hints", "token", "expires") && fields["v"] == "2" -> 2
                    else -> throw PairingException("Unsupported invitation")
                }
                val hints = if (version == 1) listOf(hint("lan", fields.getValue("address")))
                else fields.getValue("hints").split(',').map { entry ->
                    val separator = entry.indexOf(':')
                    if (separator < 1) throw PairingException("Invalid address hint")
                    hint(entry.substring(0, separator), entry.substring(separator + 1))
                }
                if (hints.isEmpty() || hints.size > 4 || hints.map { it.address }.toSet().size != hints.size)
                    throw PairingException("Invalid address hint")
                val fingerprint = fields.getValue(if (version == 1) "fingerprint" else "host_id")
                val token = fields.getValue("token")
                if (!fingerprint.matches(Regex("[0-9a-f]{64}")) || !token.matches(Regex("[A-Za-z0-9_-]{43}"))
                    || Base64.getUrlDecoder().decode(token).size != 32
                    || Base64.getUrlEncoder().withoutPadding().encodeToString(Base64.getUrlDecoder().decode(token)) != token)
                    throw PairingException("Invalid invitation")
                val expires = fields.getValue("expires")
                if (!expires.matches(Regex("[0-9]{1,19}"))) throw PairingException("Invalid expiry")
                val expiry = expires.toLong()
                require(now >= 0 && expiry > now && expiry - now <= 300)
                return PairingInvite(version, hints, fingerprint, token, expiry, admittedAt,
                    (expiry - now) * 1_000_000_000, monotonicNow).also { it.requireFresh(now) }
            } catch (error: PairingException) { throw error }
            catch (_: Exception) { throw PairingException("Invalid invitation") }
        }

        /** A literal `IP:port` / `[IPv6]:port` destination with the host-side address policy. */
        private fun hint(kind: String, address: String): AddressHint {
            if (kind !in AddressHint.KINDS) throw PairingException("Invalid address hint")
            val host: String
            val portText: String
            if (address.startsWith('[')) {
                val closing = address.indexOf(']')
                if (closing < 2 || address.getOrNull(closing + 1) != ':') throw PairingException("Invalid address")
                host = address.substring(1, closing)
                portText = address.substring(closing + 2)
                if (!host.contains(':') || host.contains('%') || !host.matches(Regex("[a-fA-F0-9:.]+"))
                    || runCatching { InetAddress.getByName(host) }.isFailure) throw PairingException("Invalid address")
            } else {
                val parts = address.split(':')
                if (parts.size != 2) throw PairingException("Invalid address")
                host = parts[0]; portText = parts[1]
                val octets = host.split('.')
                if (octets.size != 4 || octets.any { !it.matches(Regex("0|[1-9][0-9]{0,2}")) || it.toInt() > 255 })
                    throw PairingException("Invalid address")
            }
            if (!portText.matches(Regex("[0-9]{1,5}"))) throw PairingException("Invalid port")
            val port = portText.toInt()
            val ip = runCatching { InetAddress.getByName(host) }
                .getOrElse { throw PairingException("Invalid address") }
            val mappedV4 = ip.address.takeIf { bytes ->
                bytes.size == 16 && bytes.take(10).all { it == 0.toByte() }
                    && bytes[10] == 0xff.toByte() && bytes[11] == 0xff.toByte()
            }?.let { bytes -> java.net.InetAddress.getByAddress(bytes.copyOfRange(12, 16)) }
            val effective = mappedV4 ?: ip
            if (port !in 1..65535 || effective.isAnyLocalAddress || effective.isMulticastAddress || effective.isLinkLocalAddress
                || effective.hostAddress == "255.255.255.255") throw PairingException("Invalid address")
        // A tailnet hint must be a T1 overlay address; the VPN route admits nothing else.
        if (kind == "tailnet" && !zenpi.lifecycle.LanRoutePolicy.overlay(effective)) throw PairingException("Invalid address hint")
        return AddressHint(kind, host, port)
        }
    }
}
