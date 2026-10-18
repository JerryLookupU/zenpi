package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ApprovalScreenTest {
    @get:Rule val compose = createComposeRule()
    private fun sheet(matcher: SemanticsMatcher) = compose.onNode(matcher and hasAnyAncestor(hasTestTag("approval-sheet")))

    @Test fun workerApprovalsWithSameIdAreDecidedInQueueOrderAndFollowSessionReplacement() {
        val first = RemoteApproval("same", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64),
            workerSessionId = "worker-one", subtabId = "c".repeat(64))
        val second = first.copy(workerSessionId = "worker-two")
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(first, second), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        val decisions = mutableListOf<Pair<RemoteApproval, String>>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onApprove = { target, choice -> decisions.add(target to choice) })
        } } }
        // The bar offers one-tap decisions for the head only.
        compose.onNodeWithTag("Deny approval same for worker worker-two").assertDoesNotExist()
        compose.onNodeWithContentDescription("Review approvals").performClick()
        sheet(hasTestTag("approval-worker-worker-one")).assertIsDisplayed()
        compose.onNodeWithTag("approval-position").assertTextEquals("1/2")
        compose.onNodeWithText("Deny").performClick()
        org.junit.Assert.assertEquals(listOf(first to "deny"), decisions)
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(pendingApprovals = listOf(second)) })) }
        compose.onNodeWithTag("approval-position").assertTextEquals("2/2")
        sheet(hasTestTag("approval-worker-worker-two")).assertIsDisplayed()
        // A replaced worker session is a different approval: the sheet shows (and decides) the replacement only.
        val replacement = second.copy(workerSessionId = "replacement")
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(pendingApprovals = listOf(replacement)) })) }
        sheet(hasTestTag("approval-worker-replacement")).assertIsDisplayed()
        compose.onNodeWithText("Allow once").performClick()
        org.junit.Assert.assertEquals(listOf(first to "deny", replacement to "allow"), decisions)
    }

    @Test fun authorityLossRemovesApprovalAndRunningControls() {
        val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", promptControl = true,
            pendingApprovals = listOf(target), approvalControl = true,
            promptStatuses = listOf(RemotePromptStatus("running", "project", "session", "running", "TRANSIENT")))),
            "project", "session", "running", listOf(RemoteTurn("turn", "assistant", "durable display")), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        var decisions = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onApprove = { _, _ -> decisions++ }) } }
        compose.onNodeWithTag("Cancel message running").performScrollTo().assertIsEnabled()
        compose.onNodeWithTag("Allow approval approval").assertIsEnabled()
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithText("Allow once").assertIsEnabled()
        compose.runOnUiThread { state.value = AuthorityLossPolicy.displayOnly(state.value) }
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.onNodeWithTag("approval-bar").assertDoesNotExist()
        compose.onNodeWithTag("Cancel message running").assertDoesNotExist()
        compose.onNodeWithTag("Allow approval approval").assertDoesNotExist()
        compose.onNodeWithText("TRANSIENT").assertDoesNotExist()
        compose.onNodeWithText("durable display").performScrollTo().assertIsDisplayed()
        org.junit.Assert.assertEquals(0, decisions)
    }

    @Test fun pendingToolSummaryIsReadableAndNeverOffersAnUnimplementedDecision() {
        val approval = RemoteApproval("approval-" + "r".repeat(100), "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(approval))),
            "project", "session", "running", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {})
        } } }
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
        compose.onNodeWithTag("Allow approval ${approval.requestId}").assertDoesNotExist()
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithText("Awaiting approval").assertIsDisplayed()
        sheet(hasText("write_file")).assertIsDisplayed()
        sheet(hasText("Workspace write")).assertIsDisplayed()
        compose.onNodeWithTag("approval-request").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.onNodeWithText("Deny").assertDoesNotExist()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val screenshot = instrumentation.uiAutomation.takeScreenshot()
        java.io.File(instrumentation.targetContext.filesDir, "approval-summary-narrow.png").outputStream().use {
            screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        screenshot.recycle()
        // Without approval control nothing can be decided here, so the sheet may be dismissed.
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        compose.onNodeWithTag("approval-sheet").assertDoesNotExist()
        compose.onNodeWithTag("approval-bar").assertIsDisplayed()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithTag("approval-bar").assertDoesNotExist()
    }

    @Test fun sheetShowsTheColouredPatchCannotBeDismissedAndFencesAStaleSummary() {
        val preview = RemoteApprovalPreview("note.txt", "+hello\n+API_KEY=<redacted>\n" + "+bounded line\n".repeat(100),
            changed = true, truncated = true, redacted = true, withheld = false)
        val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64), preview)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        val decisions = mutableListOf<Pair<RemoteApproval, String>>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onApprove = { approval, choice -> decisions.add(approval to choice) }) } }
        compose.onNodeWithContentDescription("Review approvals").performClick()
        org.junit.Assert.assertTrue(decisions.isEmpty())
        compose.onNodeWithText("note.txt").assertIsDisplayed()
        compose.onNodeWithTag("approval-patch").assertIsDisplayed()
        compose.onNodeWithText("Sensitive values redacted").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Preview truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Allow once").assertIsDisplayed()
        // Pending approvals keep the sheet up: neither back nor a swipe dismisses it.
        InstrumentationRegistry.getInstrumentation().sendKeyDownUpSync(android.view.KeyEvent.KEYCODE_BACK)
        compose.onNodeWithTag("approval-sheet").performTouchInput { swipeDown() }
        compose.waitForIdle()
        compose.onNodeWithTag("approval-sheet").assertIsDisplayed()
        val healthy = state.value
        for (bad in listOf(target.copy(tool = "bad\n"), target.copy(preview = preview.copy(patch = "bad\u001b")),
            target.copy(preview = preview.copy(withheld = true)))) {
            compose.runOnUiThread { state.value = healthy.copy(projection = projection.copy(projects = projection.projects.map {
                it.copy(pendingApprovals = listOf(bad)) })) }
            compose.onNodeWithText("Allow once").assertIsNotEnabled()
        }
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true),
            healthy.copy(projection = projection.copy(projects = listOf(projection.projects.single().copy(pendingApprovals = listOf(target, target.copy(digest = "b".repeat(64))))))))) {
            compose.runOnUiThread { state.value = blocked }
            compose.onNodeWithText("Allow once").assertIsNotEnabled()
            compose.onNodeWithText("Deny").assertIsNotEnabled()
            org.junit.Assert.assertTrue(decisions.isEmpty())
        }
        // An ambiguous selected project fails closed: no queue, so neither bar nor sheet.
        compose.runOnUiThread { state.value = healthy.copy(projection = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))) }
        compose.onNodeWithTag("approval-sheet").assertDoesNotExist()
        compose.onNodeWithTag("approval-bar").assertDoesNotExist()
        compose.runOnUiThread { state.value = healthy }
        compose.onNodeWithContentDescription("Review approvals").performClick()
        compose.onNodeWithText("Deny").performClick()
        org.junit.Assert.assertEquals(listOf(target to "deny"), decisions)
        val withheld = target.copy(preview = RemoteApprovalPreview("<withheld>", "<withheld>", true, false, true, true))
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(pendingApprovals = listOf(withheld)) })) }
        compose.onNodeWithText("Preview withheld").assertIsDisplayed()
        compose.onNodeWithText("Sensitive values redacted").assertDoesNotExist()
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(pendingApprovals = emptyList()) })) }
        compose.onNodeWithTag("approval-sheet").assertDoesNotExist()
        org.junit.Assert.assertEquals(listOf(target to "deny"), decisions)
    }
}
