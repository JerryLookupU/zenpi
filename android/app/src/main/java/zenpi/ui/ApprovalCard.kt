package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import zenpi.pairing.ApprovalDecisionPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.transport.RemoteApproval
import zenpi.transport.RemoteProject

/** Pending tool approvals of the selected project; a decision opens [ApprovalDecisionDialog]. */
@Composable
fun ApprovalCards(state: PairingState, selected: RemoteProject?, workspaceUi: WorkspaceUiState) {
    selected?.pendingApprovals?.takeIf { state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR) }?.forEach { approval ->
        ApprovalCard(state, approval, selected.approvalControl, workspaceUi)
    }
}

@Composable
private fun ApprovalCard(state: PairingState, approval: RemoteApproval, control: Boolean, workspaceUi: WorkspaceUiState) {
    Column(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
        Text("Awaiting approval", style = MaterialTheme.typography.labelLarge)
        Text(approval.tool, style = MaterialTheme.typography.bodyMedium)
        Text(when (approval.sideEffect) { "workspace_write" -> "Workspace write"
            "command_execution" -> "Command execution"; else -> "Read only" }, style = MaterialTheme.typography.labelMedium)
        Text(approval.requestId, style = MaterialTheme.typography.labelSmall)
        approval.workerSessionId?.let { Text("Worker $it", style = MaterialTheme.typography.labelSmall) }
        if (control) {
            Row {
                val enabled = ApprovalDecisionPolicy.allows(state, approval)
                IconButton(onClick = { workspaceUi.approvalDecision = approval to "allow" }, enabled = enabled) {
                    Icon(Icons.Outlined.Check, contentDescription = "Allow approval ${approval.requestId}" + (approval.workerSessionId?.let { " for worker $it" } ?: ""))
                }
                IconButton(onClick = { workspaceUi.approvalDecision = approval to "deny" }, enabled = enabled) {
                    Icon(Icons.Outlined.Close, contentDescription = "Deny approval ${approval.requestId}" + (approval.workerSessionId?.let { " for worker $it" } ?: ""))
                }
            }
        }
    }
}

/** Confirmation of a single allow / deny decision, showing the redacted preview. */
@Composable
fun ApprovalDecisionDialog(state: PairingState, target: RemoteApproval, decision: String, workspaceUi: WorkspaceUiState, actions: ApprovalActions) {
    val enabled = ApprovalDecisionPolicy.allows(state, target)
    AlertDialog(onDismissRequest = { workspaceUi.approvalDecision = null }, title = { Text(if (decision == "allow") "Allow this tool once?" else "Deny this tool?") },
        text = { Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Text(target.tool)
            target.preview?.let { preview ->
                Text(preview.path, style = MaterialTheme.typography.labelLarge)
                if (preview.withheld) Text("Preview withheld", color = MaterialTheme.colorScheme.error)
                else Text(preview.patch, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                if (preview.redacted && !preview.withheld) Text("Sensitive values redacted", style = MaterialTheme.typography.labelSmall)
                if (preview.truncated) Text("Preview truncated", color = MaterialTheme.colorScheme.error)
            }
            Text(target.requestId, style = MaterialTheme.typography.labelSmall)
            target.workerSessionId?.let { Text("Worker $it", style = MaterialTheme.typography.labelSmall) }
            Text(target.digest.chunked(16).joinToString("\n"), style = MaterialTheme.typography.labelSmall)
        } },
        confirmButton = { TextButton(enabled = enabled, onClick = { actions.approve(target, decision); workspaceUi.approvalDecision = null }) { Text(if (decision == "allow") "Allow once" else "Deny") } },
        dismissButton = { TextButton(onClick = { workspaceUi.approvalDecision = null }) { Text("Cancel") } })
}
