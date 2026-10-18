package zenpi.pairing

import com.google.zxing.BinaryBitmap
import com.google.zxing.DecodeHintType
import com.google.zxing.NotFoundException
import com.google.zxing.Reader
import com.google.zxing.Result
import com.google.zxing.ResultMetadataType
import com.google.zxing.multi.qrcode.QRCodeMultiReader

/** Search all finder candidates; the single-symbol detector can stop at a false center. */
internal class PairingQrReader(private val baseHints: Map<DecodeHintType, *> = emptyMap<DecodeHintType, Any>()) : Reader {
    private val reader = QRCodeMultiReader()

    override fun decode(image: BinaryBitmap): Result = decode(image, baseHints)

    override fun decode(image: BinaryBitmap, hints: Map<DecodeHintType, *>?): Result {
        // Camera frames need geometric detection, never PURE_BARCODE extraction.
        val cameraHints = hints.orEmpty().filterKeys { it == DecodeHintType.NEED_RESULT_POINT_CALLBACK }
        val results = reader.decodeMultiple(image, cameraHints)
        if (results.size != 1 || results[0].resultPoints.size < 3 ||
            results[0].resultMetadata?.containsKey(ResultMetadataType.STRUCTURED_APPEND_SEQUENCE) == true) {
            throw NotFoundException.getNotFoundInstance()
        }
        return results[0]
    }

    override fun reset() = reader.reset()
}
