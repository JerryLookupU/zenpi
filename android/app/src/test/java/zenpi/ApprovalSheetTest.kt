package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsDisplayed
import androidx.compose.ui.test.hasAnyAncestor
import androidx.compose.ui.test.hasTestTag
import androidx.compose.ui.test.hasText
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithContentDescription
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performClick
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performScrollToIndex
import org.junit.Assert.assertEquals
import org.junit.Assert.assertFalse
import org.junit.Assert.assertTrue
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import zenpi.pairing.ApprovalDecisionPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.screenshot.Fixtures
import zenpi.transport.RemoteApproval
import zenpi.transport.RemoteTurn
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w360dp-h800dp-port")
class ApprovalSheetTest {
    @get:Rule val compose = createComposeRule()
    private val first = Fixtures.queued.first()
    private val second = Fixtures.queued.last()

    private fun withQueue(base: PairingState, queue: List<RemoteApproval>): PairingState = base.copy(projection = base.projection!!.copy(
        projects = base.projection!!.projects.map { if (it.id == "zenpi") it.copy(pendingApprovals = queue) else it }))

    // Robolectric does not route injected touches into the sheet's window; the semantics click runs the same handler.
    private fun tap(text: String) = compose.onNodeWithText(text).performSemanticsAction(SemanticsActions.OnClick)

    private fun recorder(decisions: MutableList<String>) = object : PairingActions {
        override fun approve(approval: RemoteApproval, decision: String) { decisions += "$decision:${approval.requestId}" }
    }

    @Test fun barStaysVisibleWhereverTheConversationIsScrolled() {
        val projection = Fixtures.connected.projection!!
        val long = Fixtures.connected.copy(projection = projection.copy(turns = projection.turns + (1..60).map { RemoteTurn("t$it", "assistant", "turn $it") }))
        compose.setContent { MaterialTheme { AppShell(long, object : PairingActions {}) } }
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
        // Reverse layout: the last index is the oldest item at the top of the conversation.
        compose.onNodeWithTag("conversation").performScrollToIndex(62)
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
        compose.onNodeWithTag("conversation").performScrollToIndex(30)
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
        compose.onNodeWithTag("conversation").performScrollToIndex(0)
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
    }

    @Test fun oneTapDecidesTheQueueHeadInOrderAndTheSheetCountsIofN() {
        val state = mutableStateOf(Fixtures.queuedState)
        val decisions = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state.value, recorder(decisions)) } }
        // Only the head is offered: the second approval has no decision control until it reaches the head.
        compose.onNodeWithContentDescription("Allow approval ${second.requestId}").assertDoesNotExist()
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithTag("approval-position").assertTextEquals("1/2")
        compose.onNodeWithTag("approval-patch").assertIsDisplayed()
        tap("Allow once")
        compose.runOnIdle { assertEquals(listOf("allow:${first.requestId}"), decisions) }
        // The host answers by dropping the head; the sheet moves to the next approval in queue order.
        compose.runOnIdle { state.value = withQueue(state.value, listOf(second)) }
        compose.onNodeWithTag("approval-position").assertTextEquals("2/2")
        compose.onNode(hasText(second.tool) and hasAnyAncestor(hasTestTag("approval-sheet"))).assertIsDisplayed()
        tap("Deny")
        compose.runOnIdle { assertEquals(listOf("allow:${first.requestId}", "deny:${second.requestId}"), decisions) }
        compose.runOnIdle { state.value = withQueue(state.value, emptyList()) }
        compose.onNodeWithTag("approval-sheet").assertDoesNotExist()
        compose.onNodeWithTag("approval-bar").assertDoesNotExist()
    }

    @Test fun staleSummaryDisablesTheButtons() {
        val state = mutableStateOf(Fixtures.queuedState)
        val decisions = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state.value, recorder(decisions)) } }
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithText("Allow once").assertIsEnabled()
        val stale = listOf(
            listOf(first, first.copy(digest = "9".repeat(64))),                          // two summaries for one approval
            listOf(first.copy(preview = first.preview!!.copy(patch = "bad\u001b"))),      // summary no longer valid
            listOf(first.copy(digest = "bad")))
        for (queue in stale) {
            compose.runOnIdle { state.value = withQueue(Fixtures.queuedState, queue) }
            compose.onNodeWithText("Allow once").assertIsNotEnabled()
            compose.onNodeWithText("Deny").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Allow approval ${first.requestId}").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = Fixtures.queuedState.copy(scanInProgress = true) }
        compose.onNodeWithText("Allow once").assertIsNotEnabled()
        compose.runOnIdle { assertTrue(decisions.isEmpty()) }
    }

    @Test fun decisionPolicyOnlyAdmitsTheCurrentHead() {
        val state = Fixtures.queuedState
        assertTrue(ApprovalDecisionPolicy.decides(state, first))
        assertFalse(ApprovalDecisionPolicy.decides(state, second))
        // The head shown on screen is refused once its digest changed underneath the tap.
        assertFalse(ApprovalDecisionPolicy.decides(withQueue(state, listOf(first.copy(digest = "9".repeat(64)), second)), first))
        assertTrue(ApprovalDecisionPolicy.decides(withQueue(state, listOf(second)), second))
        assertEquals(emptyList<RemoteApproval>(), ApprovalDecisionPolicy.queue(state.copy(phase = PairingPhase.OFFLINE)))
        assertFalse(ApprovalDecisionPolicy.decides(state.copy(phase = PairingPhase.OFFLINE), first))
    }

    @Test fun approvalsOutsideTheSelectedProjectAreNotShown() {
        val other = withQueue(Fixtures.queuedState, emptyList()).let { s -> s.copy(projection = s.projection!!.copy(
            projects = s.projection!!.projects.map { if (it.id == "docs") it.copy(pendingApprovals = listOf(first.copy(projectId = "docs", sessionId = "s-docs"))) else it })) }
        compose.setContent { MaterialTheme { AppShell(other, object : PairingActions {}) } }
        compose.onNodeWithTag("approval-bar").assertDoesNotExist()
    }
}
