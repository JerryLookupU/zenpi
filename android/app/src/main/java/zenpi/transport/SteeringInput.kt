package zenpi.transport

internal object SteeringInput {
    // Match Rust char::is_whitespace, including NEL and excluding Java-only controls.
    private fun space(code: Int): Boolean = code in 0x09..0x0d || code in setOf(0x20, 0x85, 0xa0, 0x1680,
        0x2028, 0x2029, 0x202f, 0x205f, 0x3000) || code in 0x2000..0x200a

    fun valid(text: String): Boolean {
        if (text.toByteArray(Charsets.UTF_8).size > 65536 || text.contains('\u0000')) return false
        var first = true
        var boundary = true
        var offset = 0
        while (offset < text.length) {
            val code = text.codePointAt(offset)
            if (first && !space(code)) {
                if (code == '/'.code || code == '!'.code) return false
                first = false
            }
            if (boundary && code == '@'.code) return false
            boundary = space(code)
            offset += Character.charCount(code)
        }
        return !first
    }
}
