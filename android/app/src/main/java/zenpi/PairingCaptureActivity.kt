package zenpi

import android.os.Bundle
import com.journeyapps.barcodescanner.CaptureActivity
import com.journeyapps.barcodescanner.DecoratedBarcodeView
import zenpi.pairing.PairingQrDecoderFactory

class PairingCaptureActivity : CaptureActivity() {
    override fun onCreate(savedInstanceState: Bundle?) {
        super.onCreate(savedInstanceState)
        // The camera preview shows the one-time pairing QR; keep it out of screenshots like the main screen.
        window.addFlags(android.view.WindowManager.LayoutParams.FLAG_SECURE)
        // CaptureManager installs its default factory during super.onCreate.
        // Replace it before onResume opens the camera and starts preview decoding.
        findViewById<DecoratedBarcodeView>(com.google.zxing.client.android.R.id.zxing_barcode_scanner)
            .setDecoderFactory(PairingQrDecoderFactory())
    }
}
