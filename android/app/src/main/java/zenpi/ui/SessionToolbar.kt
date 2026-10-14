package zenpi.ui

import androidx.compose.material.icons.outlined.Forum
import androidx.compose.material.icons.outlined.AddComment
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.Checklist
import androidx.compose.material.icons.outlined.Difference
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Terminal
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Text
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import zenpi.pairing.DiffReadPolicy
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.ResourceReadPolicy
import zenpi.pairing.SessionCatalogReadPolicy
import zenpi.pairing.SessionHistoryReadPolicy
import zenpi.transport.HostProjection
import zenpi.ui.nav.Route

private val activePromptStates = setOf("accepted", "running")

/**
 * The selected session's actions, at the end of the top bar's chip row: the Sessions, Diff, Tasks and
 * Terminal pages ([onRoute] for the navigation-only ones), conversation history, rename and new
 * session. Project and worktree switching live in [SwitcherDrawer].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SessionToolbar(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, actions: PairingActions, onRoute: (Route) -> Unit) {
    val state = scope.state
    val selected = scope.selected
    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Sessions") } }, state = rememberTooltipState()) {
        IconButton(onClick = { actions.sessions(-1) }, enabled = SessionCatalogReadPolicy.browse(state)) { Icon(Icons.Outlined.Forum, contentDescription = "Browse sessions") }
    }
    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
        tooltip = { PlainTooltip { Text("Browse conversation") } }, state = rememberTooltipState()) {
        IconButton(onClick = { actions.sessionHistory(false) }, enabled = SessionHistoryReadPolicy.browse(state)) {
            Icon(Icons.Outlined.History, contentDescription = "Browse conversation")
        }
    }
    if (selected?.sessionControl == true && zenpi.transport.SessionCommandCodec.mainSelected(projection)) {
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Rename session") } }, state = rememberTooltipState()) {
            IconButton(onClick = { workspaceUi.renameSessionTarget = selected.id to selected.sessionId; workspaceUi.sessionNameDraft = selected.sessionName.orEmpty() },
                enabled = state.phase == PairingPhase.CONNECTED && !state.scanInProgress && !state.resetting && !state.resetRequired
                    && zenpi.transport.SessionCatalogCodec.readable(projection)
                    && state.pendingWorkspace == null && state.pendingRevoke == null && projection.phase == "idle") {
                Icon(Icons.Outlined.Edit, contentDescription = "Rename session")
            }
        }
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("New session") } }, state = rememberTooltipState()) {
            IconButton(onClick = { workspaceUi.newSessionTarget = selected.id to selected.sessionId }, enabled = state.phase == PairingPhase.CONNECTED
                && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(projection)
                && state.pendingWorkspace == null && state.pendingRevoke == null && projection.phase == "idle"
                && selected.promptStatuses.none { it.state in activePromptStates }) {
                Icon(Icons.Outlined.AddComment, contentDescription = "New session")
            }
        }
    }
    if (selected?.diffAvailable == true) {
        IconButton(onClick = { actions.diff(".") }, enabled = DiffReadPolicy.browse(state)) {
            Icon(Icons.Outlined.Difference, contentDescription = "Review diff")
        }
    }
    if (selected != null) {
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
            tooltip = { PlainTooltip { Text("Worktree tasks") } }, state = rememberTooltipState()) {
            IconButton(onClick = { onRoute(Route.TASKS) }) { Icon(Icons.Outlined.Checklist, contentDescription = "Worktree tasks") }
        }
    }
    if (selected != null && (selected.terminal != null || selected.terminalOpenControl)) {
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
            tooltip = { PlainTooltip { Text("Desktop terminal") } }, state = rememberTooltipState()) {
            IconButton(onClick = { onRoute(Route.TERMINAL) }) { Icon(Icons.Outlined.Terminal, contentDescription = "Desktop terminal") }
        }
    }
    if (selected != null && (!selected.promptControl || selected.promptStatuses.any { it.state in activePromptStates })) {
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
            tooltip = { PlainTooltip { Text("Browse host resources") } }, state = rememberTooltipState()) {
            IconButton(onClick = { actions.resources(-1) }, enabled = ResourceReadPolicy.browse(state)) {
                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse host resources")
            }
        }
    }
    if (selected != null && (!selected.promptControl || selected.promptStatuses.any { it.state in activePromptStates })
        && projection.subtabs != null) TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
        tooltip = { PlainTooltip { Text("Browse host files") } }, state = rememberTooltipState()) {
        IconButton(onClick = { actions.files(".") }, enabled = FileReadPolicy.browse(state)) {
            Icon(Icons.Outlined.AttachFile, contentDescription = "Browse host files")
        }
    }
}
