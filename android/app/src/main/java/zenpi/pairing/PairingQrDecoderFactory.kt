package zenpi.pairing

import com.google.zxing.DecodeHintType
import com.journeyapps.barcodescanner.Decoder
import com.journeyapps.barcodescanner.DecoderFactory

internal class PairingQrDecoderFactory : DecoderFactory {
    override fun createDecoder(hints: Map<DecodeHintType, *>): Decoder = Decoder(PairingQrReader(hints))
}
