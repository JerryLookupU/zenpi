package zenpi.transport

import kotlinx.coroutines.CancellationException
import java.security.MessageDigest
import java.util.Base64
import zenpi.pairing.PairingInvite

/** One captured public identity for the invitation, signature and registration receipt. */
internal class PairingIdentityProof private constructor(val publicKey: String, val deviceId: String) {
    fun requireCurrent(read: () -> String) {
        AuthenticationProof.deviceId(deviceId) { capture(read).deviceId }
    }

    fun requireFresh(invite: PairingInvite, read: () -> String, checkActive: () -> Unit,
        now: () -> Long = { System.currentTimeMillis() / 1000 }) {
        checkActive()
        invite.requireFresh(now())
        requireCurrent(read)
        // Keystore reads may block while cancellation or invitation expiry occurs.
        checkActive()
        invite.requireFresh(now())
    }

    companion object {
        fun capture(read: () -> String): PairingIdentityProof = try {
            val public = read()
            val bytes = Base64.getUrlDecoder().decode(public)
            if (bytes.size != 65 || bytes[0] != 4.toByte()
                || Base64.getUrlEncoder().withoutPadding().encodeToString(bytes) != public) throw HostAuthenticationException()
            val id = MessageDigest.getInstance("SHA-256").digest(bytes).joinToString("") { "%02x".format(it.toInt() and 255) }
            PairingIdentityProof(public, id)
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { throw HostAuthenticationException() }
    }
}
