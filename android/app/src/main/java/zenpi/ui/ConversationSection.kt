package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingState
import zenpi.pairing.PromptControlPolicy
import zenpi.pairing.TranscriptReadPolicy
import zenpi.pairing.WorkerReadPolicy
import zenpi.pairing.WorkerStatusReadPolicy
import zenpi.pairing.WorkerTargetPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject

private val activePromptStates = setOf("accepted", "running")

/**
 * Session controls for an authorized host that have no secondary page yet: session toolbar,
 * message and task statuses, the task composer and the terminal. The conversation,
 * its composer and the model chip live in [ConversationScreen].
 */
@Composable
fun WorkspaceSection(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, workerPrompt: Draft, actions: PairingActions) {
    val state = scope.state
    val selected = scope.selected
    SessionToolbar(scope, projection, workspaceUi, actions)
    Text(projection.phase ?: "Unavailable", style = MaterialTheme.typography.labelLarge)
    PromptStatuses(state, projection, selected, actions)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        IconButton(onClick = { actions.workerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse worktree tasks")
        }
    }
    WorkerStatuses(state, projection, selected, actions)
    if (selected?.workerControl == true && scope.currentSubtabs != null) WorkerComposer(scope, workerPrompt, actions)
    TerminalSection(scope, projection, actions)
}

@Composable
private fun PromptStatuses(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: ConversationActions) {
    projection.projects.find { it.id == projection.selectedId }?.promptStatuses?.forEach { status ->
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(status.requestId.take(8), style = MaterialTheme.typography.labelSmall)
            Text(when (status.state) { "accepted" -> "Message accepted"; "running" -> "Message running"
                "succeeded" -> "Message succeeded"; "failed" -> when(status.failureCode){
                    "invalid_request"->"Message input rejected";"busy"->"Message owner busy";"resource_limit"->"Message limit reached"
                    "unauthorized","revoked_device"->"Message access denied";else->"Message failed"}; "cancelled" -> "Message cancelled"
                else -> "Message result unknown" }, style = MaterialTheme.typography.labelMedium,
                modifier=Modifier.weight(1f).padding(horizontal=8.dp),maxLines=2,overflow=TextOverflow.Ellipsis)
            if (status.state in activePromptStates && selected?.promptControl == true) {
                IconButton(onClick = { actions.cancelPrompt(status.requestId) }, enabled = PromptControlPolicy.active(state, status.requestId)) {
                    Icon(Icons.Outlined.Stop, contentDescription = "Cancel message ${status.requestId}")
                }
            }
        }
    }
}

@Composable
private fun WorkerStatuses(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: ConversationActions) {
    selected?.workerStatuses?.forEach { status ->
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text(projection.subtabs?.tabs?.find { it.id == status.subtabId }?.name ?: status.subtabId.take(12), style = MaterialTheme.typography.labelMedium)
                Text(status.requestId.take(12), style = MaterialTheme.typography.labelSmall)
                Text(when (status.state) { "accepted" -> "Task accepted"; "running" -> "Task running"; "succeeded" -> "Task succeeded"
                    "cancelled" -> "Task cancelled"; "failed" -> "Task failed"; else -> "Task result unknown" }, style = MaterialTheme.typography.bodySmall)
                if (status.state == "unknown_outcome" && status.runtimeActive) Text("Task runtime active", style = MaterialTheme.typography.labelSmall)
                status.outputText?.let { Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.fillMaxWidth()) }
                status.streamText?.let { Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.fillMaxWidth()) }
                if (status.streamTruncated) Text("Task preview truncated", style = MaterialTheme.typography.labelSmall)
                if (status.outputTruncated) Text("Task output truncated", style = MaterialTheme.typography.labelSmall)
                if (status.outputWithheld) Text("Task output withheld", style = MaterialTheme.typography.labelSmall)
            }
            if (status.state == "succeeded") IconButton(onClick = { actions.workerTranscript(status.requestId, false) },
                enabled = TranscriptReadPolicy.browse(state) && WorkerReadPolicy.projection(state, status.requestId) != null) {
                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read task output ${status.requestId}")
            }
            if (status.state == "succeeded") IconButton(onClick = { actions.workerHistory(status.requestId, false) },
                enabled = !state.workerHistoryLoading && WorkerReadPolicy.projection(state, status.requestId) != null) {
                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Read task history ${status.requestId}")
            }
            if (status.canCancel && selected.workerControl) IconButton(
                onClick = { actions.cancelWorker(status.requestId) }, enabled = WorkerTargetPolicy.cancellable(state, status.requestId) != null) {
                Icon(Icons.Outlined.Stop, contentDescription = "Cancel worktree task ${status.requestId}")
            }
        }
    }
}

@Composable
private fun WorkerComposer(scope: ScreenScope, workerPrompt: Draft, actions: ConversationActions) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val state = scope.state
    val currentSubtabs = scope.currentSubtabs ?: return
    val tab = currentSubtabs.tabs.single { it.id == currentSubtabs.activeId }
    val text = workerPrompt.value
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        OutlinedTextField(text, { if (it.toByteArray().size <= 65536) workerPrompt.value = it },
            label = { Text("Task: ${tab.name}", maxLines = 1, overflow = TextOverflow.Ellipsis) },
            modifier = Modifier.weight(1f), maxLines = 5, enabled = PromptControlPolicy.workerSend(state))
        IconButton(onClick = { keyboard?.hide(); focus.clearFocus(); workerPrompt.submit(); actions.workerPrompt(text) },
            enabled = text.isNotBlank() && !text.contains('\u0000') && !text.trimStart().startsWith("!")
                && PromptControlPolicy.workerSend(state)) {
            Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = "Send worktree task")
        }
    }
}

