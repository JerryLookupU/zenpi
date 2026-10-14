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
    @Test fun workerApprovalsWithSameIdHaveDistinctControlsAndFenceSessionReplacement() {
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
        compose.onNodeWithContentDescription("Deny approval same for worker worker-two").performScrollTo().performClick()
        compose.onNode(hasText("Worker worker-two") and hasAnyAncestor(isDialog())).assertIsDisplayed()
        compose.onNodeWithText("Deny").performClick()
        org.junit.Assert.assertEquals(listOf(second to "deny"), decisions)
        compose.onNodeWithContentDescription("Allow approval same for worker worker-one").performScrollTo().performClick()
        compose.onNode(hasText("Worker worker-one") and hasAnyAncestor(isDialog())).assertIsDisplayed()
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(pendingApprovals = listOf(first.copy(workerSessionId = "replacement"), second))
        })) }
        compose.onNodeWithText("Allow once").assertIsNotEnabled()
        org.junit.Assert.assertEquals(listOf(second to "deny"), decisions)
    }
    @Test fun authorityLossRemovesApprovalAndRunningControlsAndDisablesOpenConfirmation() {
        val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64))
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", promptControl = true,
            pendingApprovals = listOf(target), approvalControl = true,
            promptStatuses = listOf(RemotePromptStatus("running", "project", "session", "running", "TRANSIENT")))),
            "project", "session", "running", listOf(RemoteTurn("turn", "assistant", "durable display")), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        var decisions = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onApprove = { _, _ -> decisions++ }) } }
        compose.onNodeWithContentDescription("Cancel message running").performScrollTo().assertIsEnabled()
        compose.onNodeWithContentDescription("Allow approval approval").performScrollTo().performClick()
        compose.onNodeWithText("Allow once").assertIsEnabled()
        compose.runOnUiThread { state.value = AuthorityLossPolicy.displayOnly(state.value) }
        compose.onNodeWithText("Allow once").assertIsNotEnabled()
        compose.onAllNodesWithText("Cancel").filter(hasClickAction()).onLast().performClick()
        compose.onNodeWithContentDescription("Cancel message running").assertDoesNotExist()
        compose.onNodeWithContentDescription("Allow approval approval").assertDoesNotExist()
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
        compose.onNodeWithText("Awaiting approval").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("write_file").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Workspace write").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText(approval.requestId).performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Allow").assertDoesNotExist()
        compose.onNodeWithText("Deny").assertDoesNotExist()
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val screenshot = instrumentation.uiAutomation.takeScreenshot()
        java.io.File(instrumentation.targetContext.filesDir, "approval-summary-narrow.png").outputStream().use {
            screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        screenshot.recycle()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithText("Awaiting approval").assertDoesNotExist()
    }
    @Test fun approvalDecisionRequiresConfirmationAndFencesChangedDigest() {
        val preview = RemoteApprovalPreview("note.txt", "+hello\n+API_KEY=<redacted>\n" + "+bounded line\n".repeat(100),
            changed = true, truncated = true, redacted = true, withheld = false)
        val target = RemoteApproval("approval", "project", "session", "turn", "call", "write_file", "workspace_write", "agent_tool", "a".repeat(64), preview)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", pendingApprovals = listOf(target), approvalControl = true)),
            "project", "session", "running", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.ERROR, projection = projection))
        val decisions = mutableListOf<Pair<RemoteApproval, String>>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onApprove = { approval, choice -> decisions.add(approval to choice) }) } }
        compose.onNodeWithContentDescription("Allow approval approval").performScrollTo().performClick()
        org.junit.Assert.assertTrue(decisions.isEmpty())
        compose.onNodeWithText("Allow this tool once?").assertExists()
        compose.onNodeWithText("note.txt").assertIsDisplayed()
        compose.onNodeWithText("Sensitive values redacted").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Preview truncated").performScrollTo().assertIsDisplayed()
        compose.onNodeWithText("Allow once").assertIsDisplayed()
        val healthy = state.value
        for (bad in listOf(target.copy(tool = "bad\n"), target.copy(preview = preview.copy(patch = "bad\u001b")),
            target.copy(preview = preview.copy(withheld = true)))) {
            compose.runOnUiThread { state.value = healthy.copy(projection = projection.copy(projects = projection.projects.map {
                it.copy(pendingApprovals = listOf(bad)) })) }
            compose.onNodeWithText("Allow once").assertIsNotEnabled()
        }
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true),
            healthy.copy(projection = projection.copy(projects = projection.projects + projection.projects.single().copy(sessionId = "foreign"))),
            healthy.copy(projection = projection.copy(projects = listOf(projection.projects.single().copy(pendingApprovals = listOf(target, target))))))) {
            compose.runOnUiThread { state.value = blocked }
            compose.onNodeWithText("Allow once").assertIsNotEnabled()
            org.junit.Assert.assertTrue(decisions.isEmpty())
        }
        compose.runOnUiThread { state.value = healthy.copy(projection = projection.copy(sessionId = "foreign")) }
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.runOnIdle { org.junit.Assert.assertTrue(decisions.isEmpty()) }
        compose.runOnUiThread { state.value = healthy }
        compose.onNodeWithText("Allow once").assertDoesNotExist()
        compose.onNodeWithContentDescription("Allow approval approval").performScrollTo().performClick()
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(pendingApprovals = listOf(target.copy(digest = "b".repeat(64)))) })) }
        compose.onNodeWithText("Allow once").assertIsNotEnabled()
        compose.onAllNodesWithText("Cancel").filter(hasClickAction()).onLast().performClick()
        compose.runOnUiThread { state.value = state.value.copy(projection = projection) }
        compose.onNodeWithContentDescription("Deny approval approval").performScrollTo().performClick()
        compose.onNodeWithText("Deny").performClick()
        org.junit.Assert.assertEquals(listOf(target to "deny"), decisions)
        val withheld = target.copy(preview = RemoteApprovalPreview("<withheld>", "<withheld>", true, false, true, true))
        compose.runOnUiThread { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map { it.copy(pendingApprovals = listOf(withheld)) })) }
        compose.onNodeWithContentDescription("Allow approval approval").performScrollTo().performClick()
        compose.onNodeWithText("Preview withheld").assertIsDisplayed()
        compose.onNodeWithText("Sensitive values redacted").assertDoesNotExist()
        compose.runOnUiThread { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithText("Allow once").assertIsNotEnabled()
        org.junit.Assert.assertEquals(listOf(target to "deny"), decisions)
    }
}
