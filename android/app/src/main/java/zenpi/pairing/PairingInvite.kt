package zenpi.pairing

import java.net.InetAddress
import java.net.URI
import java.net.URLDecoder
import java.security.MessageDigest
import java.util.Base64

class PairingException(val reason: String) : Exception(reason)

class PairingInvite private constructor(
    val host: String, val port: Int, val fingerprint: String,
    internal val token: String, val expiresAt: Long,
    private val admittedAtNanos: Long, private val lifetimeNanos: Long, private val monotonicNow: () -> Long,
) {
    private var expired = false
    val address: String get() = if (host.contains(':')) "[$host]:$port" else "$host:$port"
    val safetyCode: String get() {
        val digest = MessageDigest.getInstance("SHA-256")
            .digest("zenpi-pair-v1:$fingerprint:$token".toByteArray(Charsets.UTF_8))
        val value = digest.take(4).fold(0L) { result, byte -> (result shl 8) or (byte.toLong() and 255) }
        return (value % 1_000_000).toString().padStart(6, '0')
    }
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
                    if (key !in setOf("v", "address", "fingerprint", "token", "expires") || fields.put(key, value) != null)
                        throw PairingException("Invalid invitation")
                }
                if (fields.size != 5 || fields["v"] != "1") throw PairingException("Unsupported invitation")
                val address = fields.getValue("address")
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
                val fingerprint = fields.getValue("fingerprint")
                val token = fields.getValue("token")
                if (!fingerprint.matches(Regex("[0-9a-f]{64}")) || !token.matches(Regex("[A-Za-z0-9_-]{43}"))
                    || Base64.getUrlDecoder().decode(token).size != 32
                    || Base64.getUrlEncoder().withoutPadding().encodeToString(Base64.getUrlDecoder().decode(token)) != token)
                    throw PairingException("Invalid invitation")
                val expires = fields.getValue("expires")
                if (!expires.matches(Regex("[0-9]{1,19}"))) throw PairingException("Invalid expiry")
                val expiry = expires.toLong()
                require(now >= 0 && expiry > now && expiry - now <= 300)
                return PairingInvite(host, port, fingerprint, token, expiry, admittedAt,
                    (expiry - now) * 1_000_000_000, monotonicNow).also { it.requireFresh(now) }
            } catch (error: PairingException) { throw error }
            catch (_: Exception) { throw PairingException("Invalid invitation") }
        }
    }
}
