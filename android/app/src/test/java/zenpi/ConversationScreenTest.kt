package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.junit4.StateRestorationTester
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performImeAction
import androidx.compose.ui.test.performTextInput
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import zenpi.screenshot.Fixtures
import zenpi.transport.RemotePromptStatus
import zenpi.transport.RemoteTurn
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w360dp-h800dp-port")
class ConversationScreenTest {
    @get:Rule val compose = createComposeRule()
    private val running = Fixtures.connected
    private val idle = running.copy(projection = running.projection!!.copy(projects = running.projection!!.projects.map {
        it.copy(promptStatuses = it.promptStatuses.map { status -> status.copy(state = "succeeded", streamText = null) })
    }, phase = "idle"))

    @Test fun draftSurvivesActivityRecreation() {
        val restoration = StateRestorationTester(compose)
        restoration.setContent { MaterialTheme { AppShell(idle, object : PairingActions {}) } }
        compose.onNodeWithTag("prompt-composer").performTextInput("unsent thought")
        restoration.emulateSavedInstanceStateRestore()
        compose.onNodeWithTag("prompt-composer").assertTextContains("unsent thought")
    }

    @Test fun composerSendsOnImeSendAndStopsOrSteersARunningMessage() {
        val state = mutableStateOf(idle)
        val sent = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state.value, object : PairingActions {
            override fun prompt(text: String) { sent += "prompt:$text" }
            override fun steer(requestId: String, text: String) { sent += "steer:$requestId:$text" }
            override fun cancelPrompt(requestId: String) { sent += "stop:$requestId" }
        }) } }
        compose.onNodeWithContentDescription("Send message").assertIsNotEnabled()
        compose.onNodeWithTag("prompt-composer").performTextInput("fix it")
        compose.onNodeWithTag("prompt-composer").performImeAction()
        compose.runOnIdle { state.value = running }
        compose.onNodeWithContentDescription("Stop message").performClick()
        compose.onNodeWithTag("prompt-composer").performTextInput("go left")
        compose.onNodeWithContentDescription("Send steering input").performClick()
        compose.runOnIdle { assertEquals(listOf("prompt:fix it", "stop:request-1", "steer:request-1:go left"), sent) }
    }

    @Test fun toolRowsStartCollapsedAndExpandOnDemand() {
        compose.setContent { MaterialTheme { AppShell(running, object : PairingActions {}) } }
        compose.onNodeWithTag("transcript-tool-call-1").assertIsDisplayed()
        compose.onNodeWithText("{\"command\":\"cargo test reconnect\"}").assertIsDisplayed()
        compose.onNodeWithContentDescription("Show arguments call-1").performClick()
        compose.onNodeWithContentDescription("Hide arguments call-1").assertIsDisplayed()
    }

    @Test fun newestTurnAndStreamingRowStayInViewWhileFollowing() {
        val state = mutableStateOf(idle)
        compose.setContent { MaterialTheme { AppShell(state.value, object : PairingActions {}) } }
        compose.onNodeWithText("run the test").assertIsDisplayed()
        val projection = idle.projection!!
        val longer = projection.copy(turns = projection.turns + (1..40).map { RemoteTurn("t$it", "assistant", "turn $it") })
        compose.runOnIdle { state.value = idle.copy(projection = longer) }
        compose.onNodeWithText("turn 40").assertIsDisplayed()
        compose.runOnIdle { state.value = running.copy(projection = longer.copy(projects = longer.projects.map {
            it.copy(promptStatuses = listOf(RemotePromptStatus("request-1", it.id, it.sessionId, "running", streamText = "Fresh stream")))
        })) }
        compose.onNodeWithText("Fresh stream").assertIsDisplayed()
    }
}
