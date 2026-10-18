package zenpi.ui.dialogs

import androidx.compose.material.icons.automirrored.outlined.ArrowLeft
import androidx.compose.material.icons.automirrored.outlined.ArrowRight
import androidx.compose.material.icons.outlined.History
import zenpi.ui.theme.asHeading
import zenpi.ui.theme.spokenAs
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.TerminalReadPolicy
import zenpi.pairing.TranscriptReadPolicy
import zenpi.pairing.WorkerHistoryNavigation
import zenpi.pairing.WorkerReadPolicy
import zenpi.pairing.WorkerStatusReadPolicy
import zenpi.pairing.WorkerTargetPolicy
import zenpi.ui.ConversationActions
import zenpi.ui.TerminalActions

/** Paged list of every worktree task of the selected project. */
@Composable
fun WorkerStatusesDialog(state: PairingState, actions: ConversationActions) {
    AlertDialog(onDismissRequest = actions::closeWorkerStatuses, title = { Text("Worktree tasks", Modifier.asHeading()) }, text = {
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (state.workerStatusesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.workerStatusesError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.workerStatusesPage?.let { page ->
                Text("${page.offset} - ${page.nextOffset} / ${page.totalTasks}", style = MaterialTheme.typography.labelSmall)
                page.tasks.forEach { task ->
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(task.requestId, style = MaterialTheme.typography.labelMedium, maxLines = 3, overflow = TextOverflow.Ellipsis,
                                modifier = Modifier.spokenAs("Worktree task"))
                            Text(task.state, style = MaterialTheme.typography.bodySmall)
                            if (task.runtimeActive) Text("Task runtime active", style = MaterialTheme.typography.labelSmall)
                        }
                        if (task.canCancel) IconButton(modifier = Modifier.testTag("Cancel listed task ${task.requestId}"), onClick = { actions.cancelWorker(task.requestId) },
                            enabled = !state.workerStatusesLoading && state.workerStatusesError == null
                                && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null
                                && WorkerTargetPolicy.cancellable(state, task.requestId) != null) {
                            Icon(Icons.Outlined.Stop, contentDescription = "Cancel listed task")
                        }
                        if (task.state == "succeeded") {
                            val readable = WorkerStatusReadPolicy.navigation(state) && WorkerReadPolicy.projection(state, task.requestId) != null
                            IconButton(modifier = Modifier.testTag("Read listed task output ${task.requestId}"), onClick = { actions.workerTranscript(task.requestId, false) }, enabled = readable && TranscriptReadPolicy.browse(state)) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read listed task output")
                            }
                            IconButton(modifier = Modifier.testTag("Read listed task history ${task.requestId}"), onClick = { actions.workerHistory(task.requestId, false) }, enabled = readable && !state.workerHistoryLoading) {
                                Icon(Icons.Outlined.History, contentDescription = "Read listed task history")
                            }
                        }
                    }
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = WorkerStatusReadPolicy.navigation(state)
        IconButton(onClick = { actions.workerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
            Icon(Icons.Outlined.Refresh, contentDescription = "Refresh task list")
        }
        IconButton(onClick = { state.workerStatusesPage?.let { page ->
            actions.workerStatuses(state.workerStatusesPrevious.lastOrNull()?.offset ?: (page.offset - 16).coerceAtLeast(0))
        } }, enabled = enabled && state.workerStatusesPrevious.isNotEmpty()) {
            Icon(Icons.AutoMirrored.Outlined.ArrowLeft, contentDescription = "Previous task list page")
        }
        IconButton(onClick = { state.workerStatusesPage?.let { actions.workerStatuses(it.nextOffset) } }, enabled = enabled && state.workerStatusesPage?.hasMore == true) {
            Icon(Icons.AutoMirrored.Outlined.ArrowRight, contentDescription = "Next task list page")
        }
    } }, dismissButton = { IconButton(onClick = actions::closeWorkerStatuses) { Icon(Icons.Outlined.Close, contentDescription = "Close task list") } })
}

/** Paged turn history of one worktree task; [scroll] outlives the dialog. */
@Composable
fun WorkerHistoryDialog(state: PairingState, requestId: String, scroll: ScrollState, actions: ConversationActions) {
    val historyEnabled = WorkerHistoryNavigation.navigation(state)
    AlertDialog(onDismissRequest = actions::closeWorkerHistory, title = { Text("Task history", Modifier.asHeading()) }, text = {
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (state.workerHistoryLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.workerHistoryError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.workerHistoryPage?.let { page ->
                Text("${page.offset} - ${page.nextOffset} / ${page.totalTurns}", style = MaterialTheme.typography.labelSmall)
                page.turns.forEach { turn ->
                    Text(turn.role, style = MaterialTheme.typography.labelSmall)
                    SelectionContainer { Text(turn.content, style = MaterialTheme.typography.bodyMedium) }
                    turn.toolCalls.forEach { call ->
                        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                            Text(call.name, modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
                            IconButton(modifier = Modifier.testTag("Read tool arguments ${call.id}"), onClick = { actions.workerToolArguments(turn.id, call.id) }, enabled = historyEnabled) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read tool arguments")
                            }
                        }
                        SelectionContainer {
                            Text(call.argumentsPreview + if (call.argumentsTruncated) "..." else "", style = MaterialTheme.typography.bodySmall,
                                fontFamily = FontFamily.Monospace)
                        }
                    }
                    if (turn.toolCallsTruncated) Text("...", style = MaterialTheme.typography.labelSmall)
                    IconButton(modifier = Modifier.testTag("Read task message ${turn.id}"), onClick = { actions.workerHistoryTurn(turn.id) }, enabled = historyEnabled) {
                        Icon(Icons.Outlined.OpenInFull, contentDescription = "Read task message")
                    }
                }
            }
        }
    }, confirmButton = { Row {
        IconButton(onClick = actions::previousWorkerHistory, enabled = historyEnabled && state.workerHistoryPrevious.isNotEmpty()) {
            Icon(Icons.AutoMirrored.Outlined.ArrowLeft, contentDescription = "Previous task history page")
        }
        IconButton(onClick = { actions.workerHistory(requestId, true) }, enabled = historyEnabled && state.workerHistoryPage?.hasMore == true) {
            Icon(Icons.AutoMirrored.Outlined.ArrowRight, contentDescription = "Next task history page")
        } } }, dismissButton = { IconButton(onClick = actions::closeWorkerHistory) { Icon(Icons.Outlined.Close, contentDescription = "Close task history") } })
}

/** Paged desktop terminal scrollback. */
@Composable
fun TerminalHistoryDialog(state: PairingState, actions: TerminalActions) {
    AlertDialog(onDismissRequest = actions::closeTerminal, title = { Text("Terminal history", Modifier.asHeading()) }, text = {
        val scroll = rememberScrollState()
        LaunchedEffect(state.terminalPage?.offset) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (state.terminalLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.terminalError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.terminalPage?.let { page ->
                Text("${page.offset} - ${page.nextOffset} / ${page.totalBytes} bytes", style = MaterialTheme.typography.labelSmall)
                SelectionContainer {
                    Text(page.text, modifier = Modifier.fillMaxWidth().testTag("terminal-history-page"), fontFamily = FontFamily.Monospace,
                        style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = TerminalReadPolicy.browse(state)
        val navigationEnabled = TerminalReadPolicy.navigation(state)
        val previous = state.terminalPage?.let { runCatching { zenpi.transport.TerminalPageCodec.previous(state.terminalPrevious, it) }.getOrNull() }
        IconButton(onClick = actions::previousTerminalPage, enabled = navigationEnabled && previous != null) {
            Icon(Icons.AutoMirrored.Outlined.ArrowLeft, contentDescription = "Previous terminal history page")
        }
        IconButton(onClick = { actions.terminalPage(true) }, enabled = navigationEnabled && state.terminalPage?.hasMore == true) {
            Icon(Icons.AutoMirrored.Outlined.ArrowRight, contentDescription = "Next terminal history page")
        }
        IconButton(onClick = { actions.terminalPage(false) }, enabled = enabled && zenpi.transport.TerminalPageCodec.terminal(state.projection) != null) { Icon(Icons.Outlined.Refresh, contentDescription = "Refresh terminal history") }
    } }, dismissButton = { IconButton(onClick = actions::closeTerminal) { Icon(Icons.Outlined.Close, contentDescription = "Close terminal history") } })
}

/** Full message or tool-argument reader for [turnId] (main session or worker). */
@Composable
fun TranscriptDialog(state: PairingState, turnId: String, actions: ConversationActions) {
    AlertDialog(onDismissRequest = actions::closeTranscript, title = { Text(state.transcriptToolCall?.name ?: "Message", Modifier.asHeading()) },
        text = { val pageScroll = rememberScrollState()
            LaunchedEffect(state.transcriptPage?.offset) { pageScroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(pageScroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.transcriptLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.transcriptError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                state.transcriptPage?.let { page ->
                    Text("${page.offset} - ${page.nextOffset} / ${page.totalBytes} bytes", style = MaterialTheme.typography.labelSmall)
                    SelectionContainer {
                        Text(page.text, modifier = Modifier.testTag("transcript-page"), style = MaterialTheme.typography.bodyMedium,
                            fontFamily = if (state.transcriptToolCall != null) FontFamily.Monospace else FontFamily.Default)
                    }
                }
            } }, confirmButton = { Row {
            val navigation = TranscriptReadPolicy.navigation(state)
            IconButton(onClick = actions::previousTranscript, enabled = TranscriptReadPolicy.previous(state) != null) {
                Icon(Icons.AutoMirrored.Outlined.ArrowLeft, contentDescription = "Previous message page")
            }
            IconButton(onClick = { if (state.transcriptWorker) actions.workerTranscript(turnId, true) else actions.transcript(turnId, true) }, enabled = navigation && state.transcriptPage?.hasMore == true) {
                Icon(Icons.AutoMirrored.Outlined.ArrowRight, contentDescription = "Next message page")
            } } }, dismissButton = { IconButton(onClick = actions::closeTranscript) { Icon(Icons.Outlined.Close, contentDescription = "Close message") } })
}
