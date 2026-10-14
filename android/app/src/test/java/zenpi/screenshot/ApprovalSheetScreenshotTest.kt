package zenpi.screenshot

import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.performClick
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.GraphicsMode
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/** The approval sheet (1/2, coloured patch) opened from the approval bar, in every portrait cell. */
@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class ApprovalSheetScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    @Test fun approvalSheet() {
        compose.setContent { ScreenshotTheme { AppShell(Fixtures.queuedState, object : PairingActions {}) } }
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.waitForIdle()
        compose.assertNoHorizontalClipping()
        compose.assertAccessibleText()
        captureScreenGolden("approval_sheet", portrait)
    }

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
