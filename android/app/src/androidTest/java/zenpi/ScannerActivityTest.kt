package zenpi

import zenpi.ui.PairingScreen
import android.Manifest
import android.content.pm.PackageManager
import android.view.accessibility.AccessibilityNodeInfo
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createAndroidComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test

class ScannerActivityTest {
    @get:Rule val compose = createAndroidComposeRule<MainActivity>()

    @Test fun actualCameraPermissionDenialReturnsToProductionPairingScreen() {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val secure = android.view.WindowManager.LayoutParams.FLAG_SECURE
        compose.runOnUiThread { assertEquals(secure, compose.activity.window.attributes.flags and secure) }
        assertNotEquals(PackageManager.PERMISSION_GRANTED,
            instrumentation.targetContext.checkSelfPermission(Manifest.permission.CAMERA))
        val monitor = instrumentation.addMonitor(PairingCaptureActivity::class.java.name, null, false)
        try {
            compose.onNodeWithText("Scan QR").performScrollTo().performClick()
            val capture = requireNotNull(instrumentation.waitForMonitorWithTimeout(monitor, 15_000)) {
                "Production scanner must launch the pairing capture activity"
            }
            instrumentation.runOnMainSync {
                val view = capture.findViewById<com.journeyapps.barcodescanner.DecoratedBarcodeView>(
                    com.google.zxing.client.android.R.id.zxing_barcode_scanner)
                assertTrue(view.decoderFactory is zenpi.pairing.PairingQrDecoderFactory)
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_phase", "production_pairing_qr_factory_verified")
            })
        } finally { instrumentation.removeMonitor(monitor) }
        val automation = instrumentation.uiAutomation
        automation.serviceInfo = automation.serviceInfo.apply {
            flags = flags or android.accessibilityservice.AccessibilityServiceInfo.FLAG_REPORT_VIEW_IDS
        }
        var denied = false
        val deadline = System.nanoTime() + 15_000_000_000L
        while (!denied && System.nanoTime() < deadline) {
            val root = automation.rootInActiveWindow
            val buttons = root?.findAccessibilityNodeInfosByViewId("com.android.permissioncontroller:id/permission_deny_button").orEmpty()
            for (button in buttons) {
                if (button.isClickable) denied = button.performAction(AccessibilityNodeInfo.ACTION_CLICK) || denied
                button.recycle()
            }
            root?.recycle()
            if (!denied) Thread.sleep(100)
        }
        assertTrue("Actual system camera permission dialog must be denied", denied)
        compose.waitUntil(15_000) { compose.onAllNodesWithText("Camera permission denied. Allow camera access in Android settings to scan.").fetchSemanticsNodes().isNotEmpty() }
        compose.onNodeWithText("Camera permission denied. Allow camera access in Android settings to scan.").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Scan QR").assertIsEnabled()
        assertNotEquals(PackageManager.PERMISSION_GRANTED,
            instrumentation.targetContext.checkSelfPermission(Manifest.permission.CAMERA))
        // FLAG_SECURE (on by default) blanks screenshots; lift it for this evidence capture only.
        compose.runOnUiThread { compose.activity.window.clearFlags(secure) }
        automation.waitForIdle(500, 5000)
        var screenshot: android.graphics.Bitmap? = null
        repeat(20) {
            if (screenshot == null) {
                val candidate = automation.takeScreenshot()
                if (candidate != null) {
                    val sample = android.graphics.Bitmap.createScaledBitmap(candidate, 32, 32, false)
                    val pixels = IntArray(1024)
                    sample.getPixels(pixels, 0, 32, 0, 0, 32, 32)
                    if (sample !== candidate) sample.recycle()
                    if (zenpi.pairing.ScreenEvidence.hasVisibleContent(pixels)) screenshot = candidate
                    else candidate.recycle()
                }
                if (screenshot == null) Thread.sleep(100)
            }
        }
        val visible = requireNotNull(screenshot) { "Permission denial screenshot must not be blank" }
        java.io.File(instrumentation.targetContext.filesDir, "scanner-permission-denied.png").outputStream().use {
            assertTrue(visible.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
        }
        visible.recycle()
        instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "scanner_permission_denial_nonblank_verified") })
    }
}
