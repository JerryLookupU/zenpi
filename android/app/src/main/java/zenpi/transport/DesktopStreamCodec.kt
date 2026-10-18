package zenpi.transport

internal object DesktopStreamCodec {
    fun decode(value: com.google.gson.JsonElement?): String? {
        if (value == null || value.isJsonNull) return null
        if (!value.isJsonPrimitive || !value.asJsonPrimitive.isString) throw WireException("Invalid desktop preview")
        return value.asString.also {
            if (it.toByteArray().size > 4096 || it.contains('\u0000')) throw WireException("Desktop preview limit")
        }
    }
}
