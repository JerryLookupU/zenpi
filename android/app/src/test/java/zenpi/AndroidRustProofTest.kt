package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Assume.assumeNotNull
import org.junit.Test
import zenpi.pairing.PairingInvite
import zenpi.security.DeviceProofCodec
import zenpi.transport.*
import java.io.*
import java.util.Base64

/** Software P256 fixture only: no Android Keystore or TLS acceptance claim. */
class AndroidRustProofTest {
    @Test fun actualPairingHandshakeAndCanonicalProofsAreSignedWithoutExportingPrivateKey() {
        val source = System.getProperty("zenpi.proof.seed")
        assumeNotNull(source)
        val seed = JsonParser.parseString(File(requireNotNull(source)).readText()).asJsonObject
        val admitted = System.nanoTime()
        val invite = PairingInvite.parse(seed["uri"].asString, monotonicNow = { System.nanoTime() - admitted })
        val generator = java.security.KeyPairGenerator.getInstance("EC")
        generator.initialize(java.security.spec.ECGenParameterSpec("secp256r1"))
        val keys = generator.generateKeyPair()
        val public = keys.public as java.security.interfaces.ECPublicKey
        fun coordinate(value: java.math.BigInteger) = value.toByteArray().takeLast(32).toByteArray().let {
            ByteArray(32 - it.size) + it
        }
        fun encode(bytes: ByteArray) = Base64.getUrlEncoder().withoutPadding().encodeToString(bytes)
        val publicBytes = byteArrayOf(4) + coordinate(public.w.affineX) + coordinate(public.w.affineY)
        val publicKey = encode(publicBytes)
        val device = java.security.MessageDigest.getInstance("SHA-256").digest(publicBytes).joinToString("") { "%02x".format(it.toInt() and 255) }
        val identityProof = PairingIdentityProof.capture { publicKey }
        assertEquals(device, identityProof.deviceId)
        fun sign(bytes: ByteArray) = encode(java.security.Signature.getInstance("SHA256withECDSA").run {
            initSign(keys.private); update(bytes); sign()
        })
        val output = ByteArrayOutputStream()
        val prelude = ByteArrayOutputStream().also { wire ->
            for (payload in listOf("""{"type":"welcome","selected_version":1}""",
                """{"type":"challenge","nonce":"${seed["nonce"].asString}","host_fingerprint":"${invite.fingerprint}"}""")) {
                FrameCodec.write(wire, JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":$payload}""").asJsonObject)
            }
        }.toByteArray()
        val input = object : InputStream() {
            private var current = ByteArrayInputStream(prelude)
            private var receipt = false
            override fun read(): Int {
                val byte = current.read()
                if (byte != -1 || receipt) return byte
                receipt = true
                val sent = ByteArrayInputStream(output.toByteArray())
                FrameCodec.read(sent)
                val request = requireNotNull(FrameCodec.read(sent))
                val response = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"${request["id"].asString}","execution_state":"succeeded","payload":{"type":"response","result":{"paired":true,"device_registered":true,"control_available":false,"device_id":"$device","device_revision":1}}}""").asJsonObject
                current = ByteArrayInputStream(ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray())
                return current.read()
            }
        }
        val name = "Phone\u2028desk\\u2028"
        val paired = PairingHandshake.exchange(input, output, invite, invite.safetyCode, name, publicKey, device,
            sign = { id -> sign(DeviceProofCodec.pair(invite.fingerprint, invite.token, id, name, publicKey)) },
            checkIdentity = { identityProof.requireCurrent { publicKey } })
        assertTrue(paired.paired); assertFalse(paired.controlAvailable)
        val results = JsonObject().apply {
            addProperty("device_id", device); addProperty("pair_wire", Base64.getEncoder().encodeToString(output.toByteArray()))
            add("proofs", com.google.gson.JsonArray().apply {
                seed["challenges"].asJsonArray.forEach { entry ->
                    val challenge = entry.asJsonObject
                    val challengeWire = ByteArrayOutputStream().also { wire ->
                        FrameCodec.write(wire, JsonParser.parseString("""{"schema_version":1,"channel":"control","payload":{"type":"challenge","nonce":"${challenge["nonce"].asString}","host_fingerprint":"${invite.fingerprint}"}}""").asJsonObject)
                    }.toByteArray()
                    val nonce = HandshakePrelude.challenge(ByteArrayInputStream(challengeWire), invite.fingerprint)
                    val id = challenge["kind"].asString
                    val bytes = when (id) {
                        "foreign_host" -> DeviceProofCodec.authenticate("b".repeat(64), nonce, device)
                        "foreign_nonce" -> DeviceProofCodec.authenticate(invite.fingerprint, "A".repeat(43), device)
                        "foreign_device" -> DeviceProofCodec.authenticate(invite.fingerprint, nonce, "c".repeat(64))
                        "revoke", "revoke_wrong_query" -> DeviceProofCodec.revokeStatus(invite.fingerprint, nonce, device, "revoke-interop", "query-interop")
                        else -> DeviceProofCodec.authenticate(invite.fingerprint, nonce, device)
                    }
                    val signature = AuthenticationProof.sign(device, { device }, {}) { sign(bytes) }
                    val revoke = id in setOf("revoke", "revoke_wrong_query")
                    val request = JsonObject().apply {
                        addProperty("schema_version", 1); addProperty("channel", "control")
                        addProperty("id", if (revoke) "query-interop" else "auth-$id")
                        add("payload", JsonObject().apply {
                            addProperty("type", if (revoke) "revoke_status" else "authenticate")
                            addProperty("device_id", device); addProperty("signature", signature)
                            if (revoke) addProperty("request_id", "revoke-interop")
                        })
                    }
                    val wire = ByteArrayOutputStream().also { FrameCodec.write(it, request) }.toByteArray()
                    add(JsonObject().apply {
                        addProperty("kind", id); addProperty("signature", signature)
                        addProperty("wire_base64", Base64.getEncoder().encodeToString(wire))
                    })
                }
            })
        }
        File(requireNotNull(System.getProperty("zenpi.proof.output"))).writeText(results.toString())
    }
}
