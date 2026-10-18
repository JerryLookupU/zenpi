package zenpi

import com.google.gson.JsonParser
import com.google.zxing.*
import com.google.zxing.common.HybridBinarizer
import com.google.zxing.qrcode.QRCodeReader
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.PairingInvite
import zenpi.pairing.PairingQrReader
import zenpi.pairing.PairingQrDecoderFactory

class PairingQrReaderTest {
    private data class Image(val pixels: ByteArray, val width: Int, val height: Int) {
        fun bitmap() = BinaryBitmap(HybridBinarizer(PlanarYUVLuminanceSource(pixels, width, height, 0, 0, width, height, false)))
        fun rotate(): Image {
            val result = ByteArray(pixels.size)
            for (y in 0 until height) for (x in 0 until width) result[x * height + height - y - 1] = pixels[y * width + x]
            return Image(result, height, width)
        }
        fun scene(): Image {
            val w = width + 120; val h = height + 100
            val result = ByteArray(w * h) { 210.toByte() }
            for (y in 0 until height) pixels.copyInto(result, (y + 60) * w + 40, y * width, (y + 1) * width)
            return Image(result, w, h)
        }
        fun double(): Image {
            val w = width * 2 + 80
            val result = ByteArray(w * height) { 255.toByte() }
            for (y in 0 until height) for (offset in listOf(0, width + 80)) {
                pixels.copyInto(result, y * w + offset, y * width, (y + 1) * width)
            }
            return Image(result, w, height)
        }
        fun perspective(): Image {
            val w = width + 140; val h = height + 120
            val transform = com.google.zxing.common.PerspectiveTransform.quadrilateralToQuadrilateral(
                45f, 30f, (w - 40).toFloat(), 65f, (w - 65).toFloat(), (h - 25).toFloat(), 70f, (h - 60).toFloat(),
                0f, 0f, width.toFloat(), 0f, width.toFloat(), height.toFloat(), 0f, height.toFloat())
            val result = ByteArray(w * h) { 210.toByte() }
            for (y in 0 until h) for (x in 0 until w) {
                val point = floatArrayOf(x + 0.5f, y + 0.5f)
                transform.transformPoints(point)
                if (point[0] >= 0 && point[1] >= 0 && point[0] < width && point[1] < height) {
                    result[y * w + x] = pixels[point[1].toInt() * width + point[0].toInt()]
                }
            }
            return Image(result, w, h)
        }
    }
    private fun fixture(): Pair<Image, com.google.gson.JsonObject> {
        val loader = requireNotNull(javaClass.classLoader)
        val bytes = requireNotNull(loader.getResourceAsStream("qr-regression/false-center.pgm")).use { it.readBytes() }
        val end = bytes.indices.filter { bytes[it] == 10.toByte() }.take(3).last() + 1
        val header = String(bytes.copyOfRange(0, end), Charsets.US_ASCII).trim().split(Regex("\\s+"))
        val metadata = requireNotNull(loader.getResourceAsStream("qr-regression/false-center.json"))
            .bufferedReader().use { JsonParser.parseReader(it).asJsonObject }
        return Image(bytes.copyOfRange(end, bytes.size), header[1].toInt(), header[2].toInt()) to metadata
    }
    @Test fun retainedProductionPixelsReproduceSingleFinderFailureAndDecodeWithCameraReader() {
        val (image, metadata) = fixture()
        assertEquals("b877ab32979ca2e0e4ae10d9b0d396560551941c8f97bd0a2251af090166a69b",
            java.security.MessageDigest.getInstance("SHA-256").digest(requireNotNull(javaClass.classLoader?.getResourceAsStream("qr-regression/false-center.pgm")).use { it.readBytes() })
                .joinToString("") { "%02x".format(it) })
        assertTrue(runCatching { QRCodeReader().decode(image.bitmap()) }.exceptionOrNull() is NotFoundException)
        val decoded = PairingQrReader().decode(image.bitmap()).text
        assertEquals(metadata["uri"].asString, decoded)
        val invite = PairingInvite.parse(decoded, metadata["now"].asLong) { 0 }
        assertEquals(metadata["uri"].asString.substringAfter("fingerprint=").take(64), invite.fingerprint)
        assertTrue(runCatching { invite.requireFresh(metadata["now"].asLong + 300) }.isFailure)
    }
    @Test fun rotatedOffCenterCameraScenesUseDetectionNotPureExtraction() {
        val (original, metadata) = fixture()
        var image = original
        repeat(4) {
            assertEquals(metadata["uri"].asString, PairingQrReader().decode(image.scene().bitmap()).text)
            assertEquals(metadata["uri"].asString, PairingQrReader().decode(image.perspective().bitmap()).text)
            image = image.rotate()
        }
    }
    @Test fun multipleSymbolsAndBlankFramesNeverChooseAnInvitation() {
        val (image, _) = fixture()
        assertTrue(runCatching { PairingQrReader().decode(image.double().bitmap()) }.exceptionOrNull() is NotFoundException)
        val blank = Image(ByteArray(400 * 400) { 255.toByte() }, 400, 400)
        assertTrue(runCatching { PairingQrReader().decode(blank.bitmap(), mapOf(DecodeHintType.PURE_BARCODE to true)) }
            .exceptionOrNull() is NotFoundException)
    }
    @Test fun cameraFactoryRunsSameReaderAndRetainsResultPointCallbacks() {
        val (image, metadata) = fixture()
        val points = mutableListOf<ResultPoint>()
        val decoder = PairingQrDecoderFactory().createDecoder(mapOf(
            DecodeHintType.NEED_RESULT_POINT_CALLBACK to ResultPointCallback { points.add(it) }))
        fun source(image: Image) = PlanarYUVLuminanceSource(image.pixels, image.width, image.height, 0, 0, image.width, image.height, false)
        assertEquals(metadata["uri"].asString, decoder.decode(source(image.scene())).text)
        assertTrue(points.size >= 3)
        assertNull(decoder.decode(source(image.double())))
        assertEquals(metadata["uri"].asString, decoder.decode(source(image.rotate())).text)
    }
}
