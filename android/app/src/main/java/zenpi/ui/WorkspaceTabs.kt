package zenpi.ui

import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.AccountTree
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Difference
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.SecondaryScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.DiffReadPolicy
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.ResourceReadPolicy
import zenpi.pairing.SessionCatalogReadPolicy
import zenpi.pairing.SessionHistoryReadPolicy
import zenpi.transport.HostProjection

private val activePromptStates = setOf("accepted", "running")

/** Project tabs, the project toolbar and the worktree / directory subtab strip. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun WorkspaceTabs(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, subtabUi: SubtabUiState,
    actions: PairingActions) {
    val state = scope.state
    ProjectTabs(scope, projection, actions)
    val selected = scope.selected
    selected?.sessionName?.let { Text(it, style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis) }
    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Sessions") } }, state = rememberTooltipState()) {
        IconButton(onClick = { actions.sessions(-1) }, enabled = SessionCatalogReadPolicy.browse(state)) { Icon(Icons.Outlined.History, contentDescription = "Browse sessions") }
    }
    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
        tooltip = { PlainTooltip { Text("Browse conversation") } }, state = rememberTooltipState()) {
        IconButton(onClick = { actions.sessionHistory(false) }, enabled = SessionHistoryReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse conversation")
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
                Icon(Icons.Outlined.Add, contentDescription = "New session")
            }
        }
    }
    if (selected?.diffAvailable == true) {
        IconButton(onClick = { actions.diff(".") }, enabled = DiffReadPolicy.browse(state)) {
            Icon(Icons.Outlined.Difference, contentDescription = "Review diff")
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
            Icon(Icons.Outlined.Folder, contentDescription = "Browse host files")
        }
    }
    SubtabStrip(scope, projection, subtabUi, actions)
    if (selected?.workspaceControl == true) {
        Row {
            IconButton(onClick = { workspaceUi.openWorkspace = true }, enabled = scope.workspaceEnabled) {
                Icon(Icons.Outlined.Add, contentDescription = "Open workspace")
            }
            IconButton(onClick = { workspaceUi.closeProject = selected.id }, enabled = scope.workspaceEnabled && projection.projects.size > 1) {
                Icon(Icons.Outlined.Close, contentDescription = "Close workspace")
            }
        }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProjectTabs(scope: ScreenScope, projection: HostProjection, actions: PairingActions) {
    val state = scope.state
    PrimaryScrollableTabRow(modifier = Modifier.testTag("workspace-tabs"), edgePadding = 0.dp,
        selectedTabIndex = projection.projects.indexOfFirst { it.id == projection.selectedId }.coerceAtLeast(0)) {
        projection.projects.forEach { project -> Tab(selected = project.id == projection.selectedId,
            modifier = Modifier.widthIn(min = 96.dp, max = 220.dp).heightIn(min = 48.dp)
                .testTag("workspace-tab-${project.id}").semantics { contentDescription = project.title },
            enabled = !state.scanInProgress && !state.resetting && !state.resetRequired
                && state.phase != PairingPhase.CONNECTING && state.pendingWorkspace == null && state.pendingRevoke == null
                && projection.projects.singleOrNull { it.id == project.id } != null,
            onClick = { if (project.workspaceControl && state.phase == PairingPhase.CONNECTED) actions.workspace("workspace_select", project.id, null) else actions.refresh(project.id) },
            text = { Text(project.title, maxLines = 2, overflow = TextOverflow.Ellipsis) }) }
    }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun SubtabStrip(scope: ScreenScope, projection: HostProjection, subtabUi: SubtabUiState, actions: WorkspaceActions) {
    val currentSubtabs = scope.currentSubtabs
    projection.subtabs?.takeIf { it.tabs.isNotEmpty() }?.let { subtabs ->
        SecondaryScrollableTabRow(modifier = Modifier.testTag("secondary-tabs"), edgePadding = 0.dp,
            selectedTabIndex = subtabs.tabs.indexOfFirst { it.id == subtabs.activeId }.coerceAtLeast(0)) {
            subtabs.tabs.forEach { tab -> Tab(selected = tab.id == subtabs.activeId,
                modifier = Modifier.widthIn(min = 96.dp, max = 220.dp).heightIn(min = 48.dp)
                    .testTag("subtab-${tab.id}").semantics { contentDescription = tab.name },
                enabled = scope.subtabEnabled && subtabs.tabs.singleOrNull { it.id == tab.id } != null,
                onClick = { actions.subtab("subtab_select", tab.id, null) }, text = { Text(tab.name, maxLines = 2, overflow = TextOverflow.Ellipsis) }) }
        }
        if (subtabs.control && currentSubtabs != null) {
            val index = subtabs.tabs.indexOfFirst { it.id == subtabs.activeId }
            val enabled = scope.subtabEnabled
            FlowRow {
                if (subtabs.worktreeControl) {
                    IconButton(enabled = enabled && subtabs.tabs.size < 32, onClick = { subtabUi.worktreeDialog = true }) {
                        Icon(Icons.Outlined.AccountTree, contentDescription = "Add Git worktree")
                    }
                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "worktree", onClick = { subtabUi.closeWorktree = subtabs.activeId }) {
                        Icon(Icons.Outlined.DeleteOutline, contentDescription = "Remove Git worktree")
                    }
                }
                if (subtabs.directoryControl) {
                    IconButton(enabled = enabled && subtabs.tabs.size < 32, onClick = { subtabUi.directoryTabDialog = true }) {
                        Icon(Icons.Outlined.Add, contentDescription = "Add directory tab")
                    }
                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "in_place", onClick = { subtabUi.closeDirectoryTab = subtabs.activeId }) {
                        Icon(Icons.Outlined.Close, contentDescription = "Close directory tab")
                    }
                }
                IconButton(enabled = enabled, onClick = {
                    subtabUi.renameSubtab = subtabs.activeId; subtabUi.subtabName = subtabs.tabs[index].name
                }) { Icon(Icons.Outlined.Edit, contentDescription = "Rename worktree tab") }
                IconButton(enabled = enabled && index > 1, onClick = {
                    actions.subtab("subtab_move", subtabs.activeId, subtabs.tabs[index - 1].id)
                }) { Icon(Icons.Outlined.ArrowBack, contentDescription = "Move worktree tab left") }
                IconButton(enabled = enabled && index > 0 && index < subtabs.tabs.lastIndex, onClick = {
                    actions.subtab("subtab_move", subtabs.activeId, subtabs.tabs.getOrNull(index + 2)?.id)
                }) { Icon(Icons.Outlined.ArrowForward, contentDescription = "Move worktree tab right") }
            }
        }
        if (subtabs.concurrencyControl && currentSubtabs != null) {
            val tab = currentSubtabs.tabs.single { it.id == currentSubtabs.activeId }
            val enabled = scope.workspaceEnabled
            Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("Concurrency", modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
                IconButton(onClick = { actions.concurrency(tab.id, tab.concurrency - 1) }, enabled = enabled && tab.concurrency > 1) {
                    Icon(Icons.Outlined.Remove, contentDescription = "Decrease worktree concurrency")
                }
                Text(tab.concurrency.toString(), modifier = Modifier.width(32.dp), style = MaterialTheme.typography.labelLarge,
                    textAlign = TextAlign.Center)
                IconButton(onClick = { actions.concurrency(tab.id, tab.concurrency + 1) }, enabled = enabled && tab.concurrency < 64) {
                    Icon(Icons.Outlined.Add, contentDescription = "Increase worktree concurrency")
                }
            }
        }
    }
}
