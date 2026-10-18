package zenpi.pairing

internal object ScreenEvidence {
    fun hasVisibleContent(pixels: IntArray): Boolean {
        if (pixels.size < 64) return false
        val visible = pixels.count { pixel ->
            ((pixel ushr 16) and 255) > 32 || ((pixel ushr 8) and 255) > 32 || (pixel and 255) > 32
        }
        return visible >= pixels.size / 10
    }
}
