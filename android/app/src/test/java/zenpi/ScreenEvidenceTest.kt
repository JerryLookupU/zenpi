package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.ScreenEvidence

class ScreenEvidenceTest {
    @Test fun blackAndNearBlackCapturesAreRejected() {
        assertFalse(ScreenEvidence.hasVisibleContent(IntArray(100) { 0xff000000.toInt() }))
        assertFalse(ScreenEvidence.hasVisibleContent(IntArray(100) { 0xff202020.toInt() }))
        assertFalse(ScreenEvidence.hasVisibleContent(IntArray(100) { if (it < 9) -1 else 0 }))
        assertFalse(ScreenEvidence.hasVisibleContent(IntArray(63) { -1 }))
    }
    @Test fun VisibleContentCanBeColoredOrLight() {
        for (color in intArrayOf(-1, 0xff00ff00.toInt(), 0xffff0000.toInt(), 0xff0000ff.toInt())) {
            assertTrue(ScreenEvidence.hasVisibleContent(IntArray(100) { if (it < 10) color else 0 }))
        }
    }
}
