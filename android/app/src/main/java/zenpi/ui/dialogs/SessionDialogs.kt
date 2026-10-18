package zenpi.ui.dialogs

import androidx.compose.material.icons.automirrored.outlined.ArrowLeft
import androidx.compose.material.icons.automirrored.outlined.ArrowRight
import zenpi.ui.theme.asHeading
import androidx.compose.ui.platform.testTag
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.SessionHistoryReadPolicy
import zenpi.ui.SessionActions
import zenpi.ui.WorkspaceUiState

/** Paged conversation history of the selected session. */
@Composable
fun SessionHistoryDialog(state: PairingState, actions: SessionActions) {
    AlertDialog(onDismissRequest = actions::closeSessionHistory,
        title = { Text("Conversation", Modifier.asHeading()) }, text = {
            val scroll = rememberScrollState()
            LaunchedEffect(state.sessionHistory?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.sessionHistoryLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.sessionHistoryError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                val enabled = SessionHistoryReadPolicy.navigation(state)
                state.sessionHistory?.let { page ->
                    Text("${page.offset}-${page.nextOffset} / ${page.totalTurns}", style = MaterialTheme.typography.labelSmall)
                    page.turns.forEach { turn ->
                        Text(turn.role, style = MaterialTheme.typography.labelSmall)
                        Text(turn.content, style = MaterialTheme.typography.bodySmall)
                        IconButton(modifier = Modifier.testTag("Read conversation message ${turn.id}"), onClick = { actions.sessionHistoryTurn(turn.id) }, enabled = enabled) {
                            Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation message")
                        }
                        turn.toolCalls.forEach { call ->
                            Text(call.name, style = MaterialTheme.typography.labelSmall)
                            Text(call.argumentsPreview, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                            IconButton(modifier = Modifier.testTag("Read conversation tool arguments ${call.id}"), onClick = { actions.sessionHistoryTool(turn.id, call.id) }, enabled = enabled) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation tool arguments")
                            }
                        }
                        HorizontalDivider()
                    }
                }
            }
        }, confirmButton = {
            val enabled = SessionHistoryReadPolicy.navigation(state)
            Row {
                IconButton(onClick = { actions.sessionHistory(false) }, enabled = SessionHistoryReadPolicy.browse(state)) {
                    Icon(Icons.Outlined.Refresh, contentDescription = "Refresh conversation")
                }
                IconButton(onClick = actions::previousSessionHistory, enabled = enabled && state.sessionHistoryPrevious.isNotEmpty()) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowLeft, contentDescription = "Previous conversation page")
                }
                IconButton(onClick = { actions.sessionHistory(true) }, enabled = enabled && state.sessionHistory?.hasMore == true) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowRight, contentDescription = "Next conversation page")
                }
            }
        }, dismissButton = { IconButton(onClick = actions::closeSessionHistory) { Icon(Icons.Outlined.Close, contentDescription = "Close conversation") } })
}

/** Confirms starting a new session for the project/session captured in [target]. */
@Composable
fun NewSessionDialog(state: PairingState, target: Pair<String, String>, workspaceUi: WorkspaceUiState, actions: SessionActions) {
    val selected = state.projection?.projects?.singleOrNull { it.id == target.first && it.sessionId == target.second && it.sessionControl }
    val enabled = selected != null && state.projection?.selectedId == target.first && state.projection?.sessionId == target.second
        && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(state.projection)
        && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null
        && state.projection?.phase == "idle" && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
    AlertDialog(onDismissRequest = { workspaceUi.newSessionTarget = null }, title = { Text("Start a new session?", Modifier.asHeading()) },
        text = { Text("The current conversation remains on the desktop. A new empty conversation will become active.") },
        confirmButton = { IconButton(onClick = { workspaceUi.newSessionTarget = null; actions.newSession() }, enabled = enabled) { Icon(Icons.Outlined.Check, contentDescription = "Confirm new session") } },
        dismissButton = { IconButton(onClick = { workspaceUi.newSessionTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel new session") } })
}

/** Renames the session captured in [target]. */
@Composable
fun RenameSessionDialog(state: PairingState, target: Pair<String, String>, workspaceUi: WorkspaceUiState, actions: SessionActions) {
    val project = state.projection?.projects?.singleOrNull { it.id == target.first && it.sessionId == target.second && it.sessionControl }
    val enabled = project != null && state.projection?.selectedId == target.first && state.projection?.sessionId == target.second
        && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(state.projection)
        && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null && state.projection?.phase == "idle"
        && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
    val draft = workspaceUi.sessionNameDraft
    val valid = runCatching { zenpi.transport.SessionCommandCodec.validateName(draft) }.isSuccess
    AlertDialog(onDismissRequest = { workspaceUi.renameSessionTarget = null }, title = { Text("Rename session", Modifier.asHeading()) }, text = {
        OutlinedTextField(value = draft, onValueChange = { workspaceUi.sessionNameDraft = it }, label = { Text("Session name") },
            singleLine = true, isError = draft.isNotEmpty() && !valid, modifier = Modifier.fillMaxWidth())
    }, confirmButton = { IconButton(onClick = { workspaceUi.renameSessionTarget = null; actions.renameSession(workspaceUi.sessionNameDraft) }, enabled = enabled && valid) {
        Icon(Icons.Outlined.Check, contentDescription = "Confirm session rename")
    } }, dismissButton = { IconButton(onClick = { workspaceUi.renameSessionTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel session rename") } })
}
