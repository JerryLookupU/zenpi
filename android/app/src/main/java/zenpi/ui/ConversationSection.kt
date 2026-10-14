package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Send
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Text
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.ModelReadPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.PromptControlPolicy
import zenpi.pairing.ResourceReadPolicy
import zenpi.pairing.TranscriptReadPolicy
import zenpi.pairing.WorkerReadPolicy
import zenpi.pairing.WorkerStatusReadPolicy
import zenpi.pairing.WorkerTargetPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.ui.dialogs.FilesDialog
import zenpi.ui.dialogs.ModelDialog
import zenpi.ui.dialogs.ResourcesDialog

private val activePromptStates = setOf("accepted", "running")

/** Everything below the header for an authorized host: tabs, approvals, conversation, composers and terminal. */
@Composable
fun WorkspaceSection(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, subtabUi: SubtabUiState,
    message: Draft, steer: Draft, workerPrompt: Draft, actions: PairingActions) {
    val state = scope.state
    val selected = scope.selected
    // The composer edits the message draft, or the steering draft while a run is active.
    val prompt = if (selected?.promptStatuses?.any { it.state in activePromptStates } == true) steer else message
    SessionToolbar(scope, projection, workspaceUi, actions)
    Text(projection.phase ?: "Unavailable", style = MaterialTheme.typography.labelLarge)
    ApprovalCards(state, selected, workspaceUi)
    PromptStatuses(state, projection, selected, actions)
    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
        IconButton(onClick = { actions.workerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse worktree tasks")
        }
        IconButton(onClick = actions::model, enabled = ModelReadPolicy.browse(state)) {
            Icon(Icons.Outlined.Tune, contentDescription = "Inspect host model")
        }
    }
    WorkerStatuses(state, projection, selected, actions)
    if (state.modelOpen) ModelDialog(state, projection, selected, actions)
    if (projection.projects.any { it.id == projection.selectedId && it.promptControl }) PromptComposer(state, projection, selected, prompt, actions)
    if (selected?.workerControl == true && scope.currentSubtabs != null) WorkerComposer(scope, workerPrompt, actions)
    if (state.resourcesOpen) ResourcesDialog(state, selected, prompt, actions)
    if (state.filesOpen) FilesDialog(state, prompt, actions)
    TranscriptTurns(state, projection, actions)
    LiveStreams(state, selected)
    TerminalSection(scope, projection, actions)
    if (projection.truncated) Text("Transcript truncated", style = MaterialTheme.typography.labelSmall)
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

/** Message / steering composer; the draft survives remote tab revisions (see [DraftKey]). */
@Composable
private fun PromptComposer(state: PairingState, projection: HostProjection, selected: RemoteProject?, prompt: Draft, actions: PairingActions) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val activePrompt = selected?.promptStatuses?.firstOrNull { it.state in activePromptStates }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        if (activePrompt == null) IconButton(onClick = { actions.resources(-1) },
            enabled = ResourceReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Select host resource")
        }
        if (activePrompt == null && projection.subtabs != null) IconButton(onClick = { actions.files(".") },
            enabled = FileReadPolicy.browse(state)) {
            Icon(Icons.Outlined.AttachFile, contentDescription = "Attach host file")
        }
        val text = prompt.value
        OutlinedTextField(text, { if (it.toByteArray().size <= 65536) prompt.value = it }, label = { Text(if (activePrompt == null) "Message" else "Steer") },
            modifier = Modifier.weight(1f).testTag("prompt-composer"), maxLines = 5,
            enabled = if (activePrompt == null) PromptControlPolicy.send(state) else PromptControlPolicy.active(state, activePrompt.requestId))
        IconButton(onClick = {
            keyboard?.hide(); focus.clearFocus(); prompt.submit()
            if (activePrompt == null) actions.prompt(text) else actions.steer(activePrompt.requestId, text)
        },
            enabled = text.isNotBlank() && !text.trimStart().startsWith("!")
                && (activePrompt == null || zenpi.transport.SteeringInput.valid(text))
                && !text.contains('\u0000')
                && (if (activePrompt == null) PromptControlPolicy.send(state) else PromptControlPolicy.active(state, activePrompt.requestId))) {
            Icon(Icons.Outlined.Send, contentDescription = if (activePrompt == null) "Send message" else "Send steering input")
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
            Icon(Icons.Outlined.Send, contentDescription = "Send worktree task")
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun TranscriptTurns(state: PairingState, projection: HostProjection, actions: ConversationActions) {
    projection.turns.forEach { turn ->
        Text(turn.role, style = MaterialTheme.typography.labelSmall)
        Text(turn.content, style = MaterialTheme.typography.bodyMedium)
        turn.toolCalls.forEach { call ->
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text(call.name, style = MaterialTheme.typography.labelMedium,
                    modifier = Modifier.weight(1f).testTag("transcript-tool-${call.id}"))
                TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                    tooltip = { PlainTooltip { Text("Read tool arguments") } }, state = rememberTooltipState()) {
                    IconButton(onClick = { actions.toolArguments(turn.id, call.id) }, enabled = TranscriptReadPolicy.tool(state, turn.id, call.id)) {
                        Icon(Icons.Outlined.OpenInFull, contentDescription = "Read tool arguments ${call.id}")
                    }
                }
            }
            Text(call.argumentsPreview, fontFamily = FontFamily.Monospace,
                style = MaterialTheme.typography.bodySmall, modifier = Modifier.fillMaxWidth())
            if (call.argumentsTruncated) Text("Arguments truncated", style = MaterialTheme.typography.labelSmall)
        }
        if (turn.toolCallsTruncated) Text("Tool list truncated", style = MaterialTheme.typography.labelSmall)
        if (turn.contentTruncated) {
            Text("Message truncated", style = MaterialTheme.typography.labelSmall)
            IconButton(onClick = { actions.transcript(turn.id, false) }, enabled = TranscriptReadPolicy.browse(state)) {
                Icon(Icons.Outlined.ArrowForward, contentDescription = "Read full message ${turn.id}")
            }
        }
        HorizontalDivider()
    }
}

/** In-flight assistant text from phone-sent messages and from the desktop. */
@Composable
private fun LiveStreams(state: PairingState, selected: RemoteProject?) {
    if (state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)) selected?.promptStatuses?.forEach { status ->
        status.streamText?.let { text ->
            Text("Assistant (in progress)", style = MaterialTheme.typography.labelSmall)
            Text(text, modifier = Modifier.testTag("live-prompt-${status.requestId}"), style = MaterialTheme.typography.bodyMedium)
            if (status.streamTruncated) Text("Preview truncated", modifier = Modifier.testTag("live-prompt-truncated-${status.requestId}"), style = MaterialTheme.typography.labelSmall)
        }
    }
    if (state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)) selected?.desktopStreamText?.let { text ->
        Text("Desktop assistant (in progress)", style = MaterialTheme.typography.labelSmall)
        Text(text, modifier = Modifier.testTag("desktop-live-text"), style = MaterialTheme.typography.bodyMedium)
    }
}
