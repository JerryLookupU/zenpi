package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.pairing.SessionCatalogReadPolicy
import zenpi.transport.SessionCatalogCodec
import zenpi.transport.SessionCommandCodec
import zenpi.ui.nav.SecondaryPage

/** Sessions page: search the host's session catalog, page through it and resume a session (confirmed). */
@Composable
fun SessionsScreen(scope: ScreenScope, actions: SessionActions, onBack: () -> Unit) {
    val state = scope.state
    var resumeTarget by remember(scope.hostScope, scope.sessionScope, state.sessions?.snapshotId, state.sessions?.offset, state.sessions?.query) { mutableStateOf<String?>(null) }
    var search by rememberSaveable(scope.hostScope, scope.sessionScope) { mutableStateOf(state.sessionsQuery.orEmpty()) }
    val searchEnabled = SessionCatalogReadPolicy.browse(state)
    val navigation = SessionCatalogReadPolicy.navigation(state)
    val submit = { resumeTarget = null; actions.searchSessions(search) }
    val searchValid = runCatching { SessionCatalogCodec.validateQuery(search.trim()) }.isSuccess
    SecondaryPage("Sessions", "Close sessions", onBack, actions = {
        IconButton(onClick = { state.sessionsPrevious.lastOrNull()?.offset?.let(actions::sessions) }, enabled = navigation && state.sessionsPrevious.isNotEmpty()) {
            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Previous sessions page")
        }
        IconButton(onClick = { state.sessions?.nextOffset?.let(actions::sessions) }, enabled = navigation && state.sessions?.nextOffset != null) {
            Icon(Icons.AutoMirrored.Outlined.ArrowForward, contentDescription = "Next sessions page")
        }
        IconButton(onClick = { actions.sessions(-1) }, enabled = searchEnabled) { Icon(Icons.Outlined.Refresh, contentDescription = "Refresh sessions") }
    }) { padding ->
        val scroll = rememberScrollState()
        LaunchedEffect(state.sessions?.offset, state.sessions?.query) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxSize().padding(padding).navigationBarsPadding().imePadding().verticalScroll(scroll).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)) {
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                OutlinedTextField(value = search, onValueChange = { search = it }, label = { Text("Search sessions") },
                    singleLine = true, enabled = searchEnabled, modifier = Modifier.weight(1f).testTag("session-search"),
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Search),
                    keyboardActions = KeyboardActions(onSearch = { if (searchEnabled && searchValid) submit() }))
                IconButton(onClick = submit, enabled = searchEnabled && searchValid) {
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
                && state.sessions?.let { SessionCatalogCodec.sameScope(it, state.projection) } == true
                && state.sessions?.let { SessionCatalogCodec.validChain(state.sessionsPrevious, it) } == true
                && state.pendingWorkspace == null && state.pendingRevoke == null && project?.sessionControl == true && state.projection?.phase == "idle"
                && SessionCommandCodec.mainSelected(state.projection)
            state.sessions?.entries?.forEach { session ->
                Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Column(Modifier.weight(1f)) {
                        Text(session.name ?: session.id, style = MaterialTheme.typography.bodyMedium, maxLines = 2, overflow = TextOverflow.Ellipsis)
                        Text(if (session.current) "Current" else "${session.turnCount} messages", style = MaterialTheme.typography.labelSmall)
                        if (session.matches > 0) Text("${session.matches} matches", style = MaterialTheme.typography.labelSmall)
                        session.snippets.forEach { snippet -> Text(snippet, style = MaterialTheme.typography.bodySmall,
                            maxLines = 3, overflow = TextOverflow.Ellipsis) }
                    }
                    IconButton(onClick = { resumeTarget = session.id }, enabled = enabled
                        && state.sessions?.let { SessionCatalogCodec.resumable(it, state.projection, session.id) } == true) {
                        Icon(Icons.Outlined.History, contentDescription = "Resume session ${session.id}")
                    }
                }
                if (resumeTarget == session.id) Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                    Text("Resume ${session.name ?: session.id}?", style = MaterialTheme.typography.labelMedium, modifier = Modifier.weight(1f))
                    IconButton(onClick = { resumeTarget = null; actions.resumeSession(session.id) }, enabled = enabled
                        && state.sessions?.let { SessionCatalogCodec.resumable(it, state.projection, session.id) } == true) {
                        Icon(Icons.Outlined.Check, contentDescription = "Confirm resume session")
                    }
                    IconButton(onClick = { resumeTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel resume session") }
                }
                HorizontalDivider()
            }
        }
    }
}
