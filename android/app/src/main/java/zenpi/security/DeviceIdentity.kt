package zenpi.security

import android.security.keystore.KeyGenParameterSpec
import android.security.keystore.KeyProperties
import java.security.KeyPairGenerator
import java.security.KeyStore
import java.security.MessageDigest
import java.security.Signature
import java.security.interfaces.ECPublicKey
import java.security.spec.ECGenParameterSpec
import java.util.Base64

/** Private signing material is generated in Android Keystore and never exported. */
class DeviceIdentity private constructor(private val alias: String) {
    private fun store(): KeyStore = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
    val publicKey: String get() {
        val key = store().getCertificate(alias).publicKey as ECPublicKey
        fun coordinate(value: java.math.BigInteger): ByteArray {
            val bytes = value.toByteArray()
            val unsigned = if (bytes.size == 33 && bytes[0] == 0.toByte()) bytes.copyOfRange(1, 33) else bytes
            require(unsigned.size <= 32)
            return ByteArray(32 - unsigned.size) + unsigned
        }
        return encode(byteArrayOf(4) + coordinate(key.w.affineX) + coordinate(key.w.affineY))
    }
    val deviceId: String get() = MessageDigest.getInstance("SHA-256").digest(Base64.getUrlDecoder().decode(publicKey))
        .joinToString("") { "%02x".format(it.toInt() and 255) }
    fun signPair(fingerprint: String, token: String, requestId: String, name: String): String {
        val message = DeviceProofCodec.pair(fingerprint, token, requestId, name, publicKey)
        val signature = Signature.getInstance("SHA256withECDSA")
        signature.initSign(store().getKey(alias, null) as java.security.PrivateKey)
        signature.update(message)
        return encode(signature.sign())
    }
    fun wipe() = synchronized(Companion) { removeKey() }
    fun wipe(expectedDeviceId: String, checkActive: () -> Unit = {}) = synchronized(Companion) {
        // A delayed revocation receipt belongs to one key, not a reusable alias.
        zenpi.transport.AuthenticationProof.current(expectedDeviceId, { deviceId }, checkActive)
        removeKey(checkActive)
    }
    private fun removeKey(checkActive: () -> Unit = {}) {
        val keys = store()
        checkActive()
        keys.deleteEntry(alias)
        check(!keys.containsAlias(alias)) { "Device key removal failed" }
    }
    fun signChallenge(fingerprint: String, nonce: String): String {
        require(fingerprint.matches(Regex("[0-9a-f]{64}")) && nonce.matches(Regex("[A-Za-z0-9_-]{43}")))
        val message = DeviceProofCodec.authenticate(fingerprint, nonce, deviceId)
        val signature = Signature.getInstance("SHA256withECDSA")
        signature.initSign(store().getKey(alias, null) as java.security.PrivateKey); signature.update(message)
        return encode(signature.sign())
    }
    fun signRevokeStatus(fingerprint: String, nonce: String, requestId: String, queryId: String): String {
        require(fingerprint.matches(Regex("[0-9a-f]{64}")) && nonce.matches(Regex("[A-Za-z0-9_-]{43}")))
        val message = DeviceProofCodec.revokeStatus(fingerprint, nonce, deviceId, requestId, queryId)
        val signature = Signature.getInstance("SHA256withECDSA")
        signature.initSign(store().getKey(alias, null) as java.security.PrivateKey); signature.update(message)
        return encode(signature.sign())
    }
    companion object {
        private fun encode(bytes: ByteArray) = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)
        @Synchronized fun loadExisting(alias: String = "zenpi.device.v1"): DeviceIdentity? {
            require(alias.matches(Regex("zenpi\\.[A-Za-z0-9._-]{1,80}")))
            return if (KeyStore.getInstance("AndroidKeyStore").apply { load(null) }.containsAlias(alias)) DeviceIdentity(alias) else null
        }
        @Synchronized fun loadOrCreate(alias: String = "zenpi.device.v1"): DeviceIdentity {
            require(alias.matches(Regex("zenpi\\.[A-Za-z0-9._-]{1,80}")))
            val store = KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
            if (!store.containsAlias(alias)) {
                KeyPairGenerator.getInstance(KeyProperties.KEY_ALGORITHM_EC, "AndroidKeyStore").apply {
                    initialize(KeyGenParameterSpec.Builder(alias, KeyProperties.PURPOSE_SIGN)
                        .setAlgorithmParameterSpec(ECGenParameterSpec("secp256r1"))
                        .setDigests(KeyProperties.DIGEST_SHA256).build())
                }.generateKeyPair()
            }
            return DeviceIdentity(alias)
        }
    }
}
