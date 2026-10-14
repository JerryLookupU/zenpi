package zenpi.ui.dialogs

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
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
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.SessionCatalogReadPolicy
import zenpi.pairing.SessionHistoryReadPolicy
import zenpi.ui.ScreenScope
import zenpi.ui.SessionActions
import zenpi.ui.WorkspaceUiState

/** Paged conversation history of the selected session. */
@Composable
fun SessionHistoryDialog(state: PairingState, actions: SessionActions) {
    AlertDialog(onDismissRequest = actions::closeSessionHistory,
        title = { Text("Conversation") }, text = {
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
                        IconButton(onClick = { actions.sessionHistoryTurn(turn.id) }, enabled = enabled) {
                            Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation message ${turn.id}")
                        }
                        turn.toolCalls.forEach { call ->
                            Text(call.name, style = MaterialTheme.typography.labelSmall)
                            Text(call.argumentsPreview, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                            IconButton(onClick = { actions.sessionHistoryTool(turn.id, call.id) }, enabled = enabled) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation tool arguments ${call.id}")
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
                    Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous conversation page")
                }
                IconButton(onClick = { actions.sessionHistory(true) }, enabled = enabled && state.sessionHistory?.hasMore == true) {
                    Icon(Icons.Outlined.ArrowForward, contentDescription = "Next conversation page")
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
    AlertDialog(onDismissRequest = { workspaceUi.newSessionTarget = null }, title = { Text("Start a new session?") },
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
    AlertDialog(onDismissRequest = { workspaceUi.renameSessionTarget = null }, title = { Text("Rename session") }, text = {
        OutlinedTextField(value = draft, onValueChange = { workspaceUi.sessionNameDraft = it }, label = { Text("Session name") },
            singleLine = true, isError = draft.isNotEmpty() && !valid, modifier = Modifier.fillMaxWidth())
    }, confirmButton = { IconButton(onClick = { workspaceUi.renameSessionTarget = null; actions.renameSession(workspaceUi.sessionNameDraft) }, enabled = enabled && valid) {
        Icon(Icons.Outlined.Check, contentDescription = "Confirm session rename")
    } }, dismissButton = { IconButton(onClick = { workspaceUi.renameSessionTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel session rename") } })
}

/** Session catalog with search and resume confirmation. */
@Composable
fun SessionsDialog(state: PairingState, scope: ScreenScope, actions: SessionActions) {
    var resumeTarget by remember(scope.hostScope, scope.sessionScope, state.sessionsOpen, state.sessions?.snapshotId, state.sessions?.offset, state.sessions?.query) { mutableStateOf<String?>(null) }
    var sessionSearch by remember(scope.hostScope, scope.sessionScope, state.sessionsOpen) { mutableStateOf(state.sessionsQuery.orEmpty()) }
    AlertDialog(onDismissRequest = actions::closeSessions, title = { Text("Sessions") }, text = {
        val scroll = rememberScrollState()
        LaunchedEffect(state.sessions?.offset) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val searchEnabled = SessionCatalogReadPolicy.browse(state)
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(value = sessionSearch, onValueChange = { sessionSearch = it }, label = { Text("Search sessions") },
                    singleLine = true, enabled = searchEnabled, modifier = Modifier.weight(1f).testTag("session-search"))
                IconButton(onClick = { resumeTarget = null; actions.searchSessions(sessionSearch) }, enabled = searchEnabled
                    && runCatching { zenpi.transport.SessionCatalogCodec.validateQuery(sessionSearch.trim()) }.isSuccess) {
                    Icon(Icons.Outlined.Search, contentDescription = "Search sessions")
                }
            }
            state.sessions?.query?.let { Text(it, style = MaterialTheme.typography.labelSmall) }
            if (!state.sessionsLoading && state.sessionsError == null && state.sessions?.entries?.isEmpty() == true)
                Text("No sessions", style = MaterialTheme.typography.bodySmall)
            if (state.sessionsLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.sessionsError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            val project = state.projection?.projects?.singleOrNull { it.id == state.projection.selectedId }
            val enabled = state.phase == PairingPhase.CONNECTED && !state.sessionsLoading && state.sessionsError == null
                && !state.scanInProgress && !state.resetting && !state.resetRequired
                && state.sessions?.let { zenpi.transport.SessionCatalogCodec.sameScope(it, state.projection) } == true
                && state.sessions?.let { zenpi.transport.SessionCatalogCodec.validChain(state.sessionsPrevious, it) } == true
                && state.pendingWorkspace == null && state.pendingRevoke == null && project?.sessionControl == true && state.projection?.phase == "idle"
                && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
            state.sessions?.entries?.forEach { session ->
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(session.name ?: session.id, style = MaterialTheme.typography.bodySmall, maxLines = 2, overflow = TextOverflow.Ellipsis)
                        Text(if (session.current) "Current" else "${session.turnCount} messages", style = MaterialTheme.typography.labelSmall)
                        if (session.matches > 0) Text("${session.matches} matches", style = MaterialTheme.typography.labelSmall)
                        session.snippets.forEach { snippet -> Text(snippet, style = MaterialTheme.typography.bodySmall,
                            maxLines = 3, overflow = TextOverflow.Ellipsis) }
                    }
                    IconButton(onClick = { resumeTarget = session.id }, enabled = enabled
                        && state.sessions?.let { zenpi.transport.SessionCatalogCodec.resumable(it, state.projection, session.id) } == true) {
                        Icon(Icons.Outlined.History, contentDescription = "Resume session ${session.id}")
                    }
                }
                HorizontalDivider()
            }
            resumeTarget?.let { target ->
                Text("Resume $target?", style = MaterialTheme.typography.labelMedium)
                Row {
                    IconButton(onClick = { resumeTarget = null; actions.resumeSession(target) }, enabled = enabled
                        && state.sessions?.let { zenpi.transport.SessionCatalogCodec.resumable(it, state.projection, target) } == true) { Icon(Icons.Outlined.Check, contentDescription = "Confirm resume session") }
                    IconButton(onClick = { resumeTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel resume session") }
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = SessionCatalogReadPolicy.browse(state)
        val navigation = SessionCatalogReadPolicy.navigation(state)
        IconButton(onClick = { state.sessionsPrevious.lastOrNull()?.offset?.let(actions::sessions) }, enabled = navigation && state.sessionsPrevious.isNotEmpty()) {
            Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous sessions page")
        }
        IconButton(onClick = { state.sessions?.nextOffset?.let(actions::sessions) }, enabled = navigation && state.sessions?.nextOffset != null) {
            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next sessions page")
        }
        IconButton(onClick = { actions.sessions(-1) }, enabled = enabled) { Icon(Icons.Outlined.Refresh, contentDescription = "Refresh sessions") }
    } }, dismissButton = { IconButton(onClick = actions::closeSessions) { Icon(Icons.Outlined.Close, contentDescription = "Close sessions") } })
}
