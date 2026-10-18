package zenpi.ui

import androidx.compose.material.icons.automirrored.outlined.List
import androidx.compose.material.icons.outlined.History
import zenpi.ui.theme.spokenAs
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
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

import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material3.Surface
import zenpi.ui.nav.SecondaryPage

/**
 * Tasks page: the selected project's worktree tasks (live status, output, history and cancel), the
 * paged list of every task, and the task composer for the active worktree pinned above the keyboard.
 */
@Composable
fun TasksScreen(scope: ScreenScope, workerPrompt: Draft, actions: ConversationActions, onBack: () -> Unit) {
    val state = scope.state
    val selected = scope.selected
    val projection = state.projection
    SecondaryPage("Worktree tasks", "Back to conversation", onBack, actions = {
        IconButton(onClick = { actions.workerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.List, contentDescription = "Browse worktree tasks")
        }
    }, bottomBar = {
        if (selected?.workerControl == true && scope.currentSubtabs != null) Surface(tonalElevation = 3.dp) {
            Column(Modifier.navigationBarsPadding().imePadding().padding(horizontal = 12.dp, vertical = 8.dp)) { WorkerComposer(scope, workerPrompt, actions) }
        }
    }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
            if (selected?.workerStatuses.isNullOrEmpty()) Text("No worktree tasks", style = MaterialTheme.typography.bodyMedium)
            if (projection != null) WorkerStatuses(state, projection, selected, actions)
        }
    }
}

@Composable
private fun WorkerStatuses(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: ConversationActions) {
    selected?.workerStatuses?.forEach { status ->
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                val tab = projection.subtabs?.tabs?.find { it.id == status.subtabId }?.name
                Text(tab ?: status.subtabId.take(12), style = MaterialTheme.typography.labelMedium,
                    modifier = if (tab == null) Modifier.spokenAs("Worktree") else Modifier)
                Text(status.requestId.take(12), style = MaterialTheme.typography.labelSmall, modifier = Modifier.spokenAs(null))
                Text(when (status.state) { "accepted" -> "Task accepted"; "running" -> "Task running"; "succeeded" -> "Task succeeded"
                    "cancelled" -> "Task cancelled"; "failed" -> "Task failed"; else -> "Task result unknown" }, style = MaterialTheme.typography.bodySmall)
                if (status.state == "unknown_outcome" && status.runtimeActive) Text("Task runtime active", style = MaterialTheme.typography.labelSmall)
                status.outputText?.let { Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.fillMaxWidth()) }
                status.streamText?.let { Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.fillMaxWidth()) }
                if (status.streamTruncated) Text("Task preview truncated", style = MaterialTheme.typography.labelSmall)
                if (status.outputTruncated) Text("Task output truncated", style = MaterialTheme.typography.labelSmall)
                if (status.outputWithheld) Text("Task output withheld", style = MaterialTheme.typography.labelSmall)
            }
            if (status.state == "succeeded") IconButton(modifier = Modifier.testTag("Read task output ${status.requestId}"), onClick = { actions.workerTranscript(status.requestId, false) },
                enabled = TranscriptReadPolicy.browse(state) && WorkerReadPolicy.projection(state, status.requestId) != null) {
                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read task output")
            }
            if (status.state == "succeeded") IconButton(modifier = Modifier.testTag("Read task history ${status.requestId}"), onClick = { actions.workerHistory(status.requestId, false) },
                enabled = !state.workerHistoryLoading && WorkerReadPolicy.projection(state, status.requestId) != null) {
                Icon(Icons.Outlined.History, contentDescription = "Read task history")
            }
            if (status.canCancel && selected.workerControl) IconButton(modifier = Modifier.testTag("Cancel worktree task ${status.requestId}"), 
                onClick = { actions.cancelWorker(status.requestId) }, enabled = WorkerTargetPolicy.cancellable(state, status.requestId) != null) {
                Icon(Icons.Outlined.Stop, contentDescription = "Cancel worktree task")
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
