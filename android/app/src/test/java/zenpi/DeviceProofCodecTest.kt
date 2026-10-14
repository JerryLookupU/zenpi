package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.DeviceProofCodec

class DeviceProofCodecTest {
    @Test fun sharedRustVectorsMatchExactSignedUtf8IncludingUnicodeAndLiteralEscapes() {
        val input = requireNotNull(javaClass.getResourceAsStream("/device-proof-v1.json"))
        val vectors = input.reader(Charsets.UTF_8).use { JsonParser.parseReader(it).asJsonArray }
        for (entry in vectors) {
            val value = entry.asJsonObject
            val fields = value["fields"].asJsonArray.map { it.asString }
            val actual = when (value["kind"].asString) {
                "pair" -> DeviceProofCodec.pair(fields[0], fields[1], fields[2], fields[3], fields[4])
                "authenticate" -> DeviceProofCodec.authenticate(fields[0], fields[1], fields[2])
                "revoke_status" -> DeviceProofCodec.revokeStatus(fields[0], fields[1], fields[2], fields[3], fields[4])
                else -> error("Unknown vector")
            }
            assertArrayEquals(value["message"].asString.toByteArray(Charsets.UTF_8), actual)
        }
    }
    @Test fun literalBackslashEscapeAndUnicodeSeparatorHaveDifferentProofs() {
        val literal = DeviceProofCodec.pair("f", "t", "r", "\\u2028", "p")
        val unicode = DeviceProofCodec.pair("f", "t", "r", "\u2028", "p")
        assertFalse(literal.contentEquals(unicode))
        assertEquals("\\u2028", JsonParser.parseString(literal.toString(Charsets.UTF_8)).asJsonArray[4].asString)
        assertEquals("\u2028", JsonParser.parseString(unicode.toString(Charsets.UTF_8)).asJsonArray[4].asString)
        val escaped = com.google.gson.GsonBuilder().disableHtmlEscaping().create().toJson(
            listOf("zenpi-device-pair-v1", "f", "t", "r", "\u2028", "p")).toByteArray(Charsets.UTF_8)
        assertFalse(escaped.contentEquals(unicode))
    }
    @Test fun actualP256SignatureBindsExactCanonicalBytesNotEquivalentJson() {
        val generator = java.security.KeyPairGenerator.getInstance("EC")
        generator.initialize(java.security.spec.ECGenParameterSpec("secp256r1"))
        val key = generator.generateKeyPair()
        val message = DeviceProofCodec.pair("f", "t", "r", "Phone\u2028desk", "p")
        val signature = java.security.Signature.getInstance("SHA256withECDSA").run {
            initSign(key.private); update(message); sign()
        }
        fun accepts(bytes: ByteArray): Boolean = java.security.Signature.getInstance("SHA256withECDSA").run {
            initVerify(key.public); update(bytes); this.verify(signature)
        }
        assertTrue(accepts(message))
        val gson = com.google.gson.GsonBuilder().disableHtmlEscaping().create().toJson(
            listOf("zenpi-device-pair-v1", "f", "t", "r", "Phone\u2028desk", "p"))
        assertFalse(accepts(gson.toByteArray(Charsets.UTF_8)))
        assertFalse(accepts(DeviceProofCodec.pair("f", "t", "another", "Phone\u2028desk", "p")))
    }
}
