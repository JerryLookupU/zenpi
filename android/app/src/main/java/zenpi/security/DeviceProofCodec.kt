package zenpi.security

import com.google.gson.GsonBuilder

internal object DeviceProofCodec {
    private val gson = GsonBuilder().disableHtmlEscaping().create()
    fun pair(fingerprint: String, token: String, request: String, name: String, publicKey: String): ByteArray =
        encode(listOf("zenpi-device-pair-v1", fingerprint, token, request, name, publicKey))
    fun authenticate(fingerprint: String, nonce: String, device: String): ByteArray =
        encode(listOf("zenpi-device-auth-v1", fingerprint, nonce, device))
    fun revokeStatus(fingerprint: String, nonce: String, device: String, request: String, query: String): ByteArray =
        encode(listOf("zenpi-device-revoke-status-v1", fingerprint, nonce, device, request, query))

    private fun encode(values: List<String>): ByteArray {
        // Gson escapes JS line separators; serde_json signs their literal UTF-8.
        val json = gson.toJson(values)
        val canonical = StringBuilder(json.length)
        var index = 0
        while (index < json.length) {
            if (json[index] == '\\' && index + 1 < json.length) {
                val escape = json.substring(index, minOf(index + 6, json.length))
                if (escape == "\\u2028" || escape == "\\u2029") {
                    canonical.append(if (escape == "\\u2028") '\u2028' else '\u2029'); index += 6
                } else {
                    // Consume escaped backslash/quote as a pair, not a second escape start.
                    canonical.append(json[index]).append(json[index + 1]); index += 2
                }
            } else { canonical.append(json[index]); index++ }
        }
        return canonical.toString().toByteArray(Charsets.UTF_8)
    }
}
