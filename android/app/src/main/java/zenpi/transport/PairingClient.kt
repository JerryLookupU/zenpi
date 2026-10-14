package zenpi.transport

import kotlinx.coroutines.*
import zenpi.pairing.PairingInvite
import zenpi.security.DeviceIdentity
import java.net.InetSocketAddress
import java.net.Socket
import java.security.MessageDigest
import java.security.SecureRandom
import java.security.cert.CertificateException
import java.security.cert.X509Certificate
import javax.net.ssl.*

data class PairingResult(val paired: Boolean, val controlAvailable: Boolean, val deviceRegistered: Boolean, val deviceId: String)

/**
 * Pins the host key: `host_id = sha256(SPKI)` (QR v2), so a new certificate for the
 * same key still verifies. A v1 pin, `sha256(certificate DER)`, is still honoured so
 * saved hosts keep working until they migrate to the host id.
 */
class PinnedTrustManager internal constructor(fingerprint: String, private val now: () -> java.util.Date) : X509TrustManager {
    constructor(fingerprint: String) : this(fingerprint, { java.util.Date() })
    private val expected = if (fingerprint.matches(Regex("[0-9a-f]{64}")))
        fingerprint.chunked(2).map { it.toInt(16).toByte() }.toByteArray()
    else throw HostIdentityException()
    /** The verified certificate's host id and v1 pin; set only after a successful check. */
    @Volatile var verified: VerifiedHost? = null
        private set
    override fun getAcceptedIssuers(): Array<X509Certificate> = emptyArray()
    override fun checkClientTrusted(chain: Array<X509Certificate>?, authType: String?) {
        throw CertificateException("Client trust unavailable")
    }
    override fun checkServerTrusted(chain: Array<X509Certificate>?, authType: String?) {
        if (chain == null || chain.size != 1) throw HostIdentityException()
        try { chain[0].checkValidity(now()) }
        catch (error: CertificateException) { throw HostIdentityException(error) }
        val host = try { VerifiedHost.of(chain[0]) }
        catch (error: CertificateException) { throw HostIdentityException(error) }
        if (!MessageDigest.isEqual(host.hostId.hexBytes(), expected) && !MessageDigest.isEqual(host.legacyPin.hexBytes(), expected))
            throw HostIdentityException()
        verified = host
    }
}

/** Both names of one verified certificate: the key's host id and the v1 certificate pin. */
data class VerifiedHost(val hostId: String, val legacyPin: String) {
    /** The challenge may name the host either way; both are bound to the verified certificate. */
    val names: Set<String> get() = setOf(hostId, legacyPin)
    companion object {
        private fun sha256(bytes: ByteArray) = MessageDigest.getInstance("SHA-256").digest(bytes)
            .joinToString("") { "%02x".format(it.toInt() and 255) }
        fun of(certificate: X509Certificate) = VerifiedHost(sha256(certificate.publicKey.encoded), sha256(certificate.encoded))
    }
}

private fun String.hexBytes() = chunked(2).map { it.toInt(16).toByte() }.toByteArray()

class PairingClient(private val bindSocket: (Socket, String) -> Unit = { _, _ -> }) {
    suspend fun pair(invite: PairingInvite, deviceName: String, identity: DeviceIdentity): PairingResult {
        invite.requireFresh()
        if (deviceName.isBlank() || deviceName.toByteArray().size > 128
            || deviceName.any { Character.isISOControl(it) }) throw WireException("Invalid device name")
        val proof = withContext(Dispatchers.IO) { PairingIdentityProof.capture { identity.publicKey } }
        val active = SocketLease()
        return coroutineScope {
            // Blocking socket I/O must close immediately on cancellation, including connect/handshake.
            val closer = launch(start = CoroutineStart.UNDISPATCHED) {
                try { awaitCancellation() } finally { active.close() }
            }
            val deadline = launch { delay(15_000); active.close() }
            try {
                OperationDeadline.run {
                    withContext(Dispatchers.IO) {
                        fun current() = proof.requireFresh(invite, { identity.publicKey }, { ensureActive() })
                        val raw = Socket()
                        try {
                            active.install(raw)
                            current()
                            bindSocket(raw, invite.host)
                            current()
                            raw.connect(InetSocketAddress(invite.host, invite.port), 5000)
                            current()
                            val context = SSLContext.getInstance("TLSv1.3")
                            context.init(null, arrayOf(PinnedTrustManager(invite.fingerprint)), SecureRandom())
                            val socket = context.socketFactory.createSocket(raw, "zenpi.local", invite.port, true) as SSLSocket
                            active.transfer(raw, socket)
                            socket.use {
                                socket.enabledProtocols = arrayOf("TLSv1.3")
                                socket.sslParameters = socket.sslParameters.apply { applicationProtocols = arrayOf("zenpi/1") }
                                socket.soTimeout = 5000
                                socket.startHandshake()
                                HandshakePrelude.applicationProtocol(socket.applicationProtocol)
                                PairingHandshake.exchange(socket.inputStream, socket.outputStream, invite,
                                    deviceName, proof.publicKey, proof.deviceId,
                                    sign = { id -> identity.signPair(invite.fingerprint, invite.token, id, deviceName) },
                                    checkActive = { ensureActive() }, checkIdentity = { proof.requireCurrent { identity.publicKey } })
                            }
                        } finally { try { active.close() } finally { raw.close() } }
                    }
                }
            } finally { deadline.cancelAndJoin(); closer.cancelAndJoin() }
        }
    }
}
