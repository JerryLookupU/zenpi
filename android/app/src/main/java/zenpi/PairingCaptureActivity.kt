package zenpi

import android.os.Bundle
import com.journeyapps.barcodescanner.CaptureActivity
import com.journeyapps.barcodescanner.DecoratedBarcodeView
import zenpi.pairing.PairingQrDecoderFactory

class PairingCaptureActivity : CaptureActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // CaptureManager installs its default factory during super.onCreate.
        // Replace it before onResume opens the camera and starts preview decoding.
        findViewById<DecoratedBarcodeView>(com.google.zxing.client.android.R.id.zxing_barcode_scanner)
            .setDecoderFactory(PairingQrDecoderFactory())
    }
}
