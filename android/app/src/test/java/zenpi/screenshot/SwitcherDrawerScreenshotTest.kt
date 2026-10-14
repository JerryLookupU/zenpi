package zenpi.screenshot

import androidx.compose.ui.test.assertIsSelected
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.performClick
import org.junit.Assert.assertEquals
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.GraphicsMode
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/** The drawer inbox opened from the top bar, in every portrait cell. */
@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class SwitcherDrawerScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    @Test fun drawerInbox() {
        val selected = mutableListOf<String>()
        compose.setContent { ScreenshotTheme { AppShell(Fixtures.connected, object : PairingActions {
            override fun workspace(action: String, projectId: String, directory: String?) { selected += "$action:$projectId" }
        }) } }
        compose.onNodeWithContentDescription("Open switcher, 1 pending approval").performClick()
        compose.waitForIdle()
        compose.onNodeWithTag("inbox-zenpi").assertIsSelected()
        compose.assertNoHorizontalClipping()
        compose.assertAccessibleText()
        compose.captureGolden("drawer", portrait)
        compose.onNodeWithContentDescription("infra, idle").performClick()
        compose.waitForIdle()
        assertEquals(listOf("workspace_select:infra"), selected)
        // Choosing a session closes the drawer: its rows slide wholly off-screen.
        assertTrue(compose.onNodeWithTag("inbox-infra").fetchSemanticsNode().positionInRoot.x < 0f)
    }

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
