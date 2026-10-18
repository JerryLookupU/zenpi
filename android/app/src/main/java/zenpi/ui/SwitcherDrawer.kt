package zenpi.ui

import androidx.compose.material.icons.outlined.FolderOpen
import androidx.compose.material.icons.outlined.FolderOff
import androidx.compose.material.icons.automirrored.outlined.CallSplit
import androidx.compose.material.icons.outlined.CreateNewFolder
import androidx.compose.material.icons.outlined.RemoveCircleOutline
import androidx.compose.material.icons.outlined.KeyboardDoubleArrowLeft
import androidx.compose.material.icons.outlined.KeyboardDoubleArrowRight
import zenpi.ui.theme.asHeading
import androidx.compose.foundation.background
import androidx.compose.foundation.clickable
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.shape.RoundedCornerShape
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material3.Badge
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PrimaryScrollableTabRow
import androidx.compose.material3.SecondaryScrollableTabRow
import androidx.compose.material3.Tab
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.selected
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject

/** Inbox sections, in display order: what blocks on the user first, then live work, then the rest. */
enum class InboxGroup(val label: String) { NEEDS_YOU("Needs you"), RUNNING("Running"), IDLE("Idle") }

/** Groups the host's sessions (one per project) for the drawer inbox. */
object SwitcherInbox {
    private val active = setOf("accepted", "running")

    fun group(project: RemoteProject): InboxGroup = when {
        project.pendingApprovals.isNotEmpty() -> InboxGroup.NEEDS_YOU
        project.promptStatuses.any { it.state in active } || project.workerStatuses.any { it.state in active }
            || project.desktopStreamText != null -> InboxGroup.RUNNING
        else -> InboxGroup.IDLE
    }

    /** Non-empty groups in [InboxGroup] order; projects keep their host order within a group. */
    fun groups(projection: HostProjection?): List<Pair<InboxGroup, List<RemoteProject>>> =
        projection?.projects.orEmpty().groupBy(::group).toSortedMap().map { it.key to it.value }

    /** The drawer badge: approvals waiting on this device across every project. */
    fun pendingApprovals(projection: HostProjection?): Int = projection?.projects.orEmpty().sumOf { it.pendingApprovals.size }
}

/** Whether [project] can be switched to now; the same fence guards inbox rows and project tabs. */
private fun switchable(scope: ScreenScope, projection: HostProjection, project: RemoteProject): Boolean {
    val state = scope.state
    return !state.scanInProgress && !state.resetting && !state.resetRequired && state.phase != PairingPhase.CONNECTING
        && state.pendingWorkspace == null && state.pendingRevoke == null && projection.projects.singleOrNull { it.id == project.id } != null
}

private fun switchTo(scope: ScreenScope, project: RemoteProject, actions: PairingActions) {
    if (project.workspaceControl && scope.state.phase == PairingPhase.CONNECTED) actions.workspace("workspace_select", project.id, null)
    else actions.refresh(project.id)
}

/**
 * The switcher drawer: the session inbox grouped by [InboxGroup], then project and worktree
 * management (project tabs, open / close workspace, worktree tabs and their controls).
 * [onSwitched] runs after an inbox row is chosen so a modal host can close itself.
 */
@Composable
fun SwitcherDrawer(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, subtabUi: SubtabUiState,
    actions: PairingActions, onSwitched: () -> Unit = {}) {
    Inbox(scope, projection, actions, onSwitched)
    HorizontalDivider()
    Text("Projects", style = MaterialTheme.typography.titleSmall, modifier = Modifier.asHeading())
    ProjectTabs(scope, projection, actions)
    val selected = scope.selected
    if (selected?.workspaceControl == true) {
        Row {
            IconButton(onClick = { workspaceUi.openWorkspace = true }, enabled = scope.workspaceEnabled) {
                Icon(Icons.Outlined.FolderOpen, contentDescription = "Open workspace")
            }
            IconButton(onClick = { workspaceUi.closeProject = selected.id }, enabled = scope.workspaceEnabled && projection.projects.size > 1) {
                Icon(Icons.Outlined.FolderOff, contentDescription = "Close workspace")
            }
        }
    }
    if (projection.subtabs?.tabs?.isNotEmpty() == true) Text("Worktrees", style = MaterialTheme.typography.titleSmall, modifier = Modifier.asHeading())
    SubtabStrip(scope, projection, subtabUi, actions)
}

@Composable
private fun Inbox(scope: ScreenScope, projection: HostProjection, actions: PairingActions, onSwitched: () -> Unit) {
    SwitcherInbox.groups(projection).forEach { (group, projects) ->
        Text(group.label, style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary,
            modifier = Modifier.padding(top = 8.dp).asHeading())
        projects.forEach { project ->
            InboxRow(project, group, project.id == projection.selectedId, switchable(scope, projection, project)) {
                switchTo(scope, project, actions); onSwitched()
            }
        }
    }
}

/** One session row. Its children are replaced by a single spoken summary (title, group, approvals). */
@Composable
private fun InboxRow(project: RemoteProject, group: InboxGroup, selected: Boolean, enabled: Boolean, onClick: () -> Unit) {
    val approvals = project.pendingApprovals.size
    val summary = listOfNotNull(project.title, group.label.lowercase(),
        approvals.takeIf { it > 0 }?.let { if (it == 1) "1 approval" else "$it approvals" }).joinToString(", ")
    Row(Modifier.fillMaxWidth().heightIn(min = 48.dp).testTag("inbox-${project.id}")
        .background(if (selected) MaterialTheme.colorScheme.secondaryContainer else Color.Transparent, RoundedCornerShape(24.dp))
        .clickable(enabled = enabled, role = Role.Tab, onClick = onClick)
        .clearAndSetSemantics { contentDescription = summary; this.selected = selected }
        .padding(horizontal = 16.dp, vertical = 8.dp),
        verticalAlignment = Alignment.CenterVertically, horizontalArrangement = Arrangement.spacedBy(12.dp)) {
        Column(Modifier.weight(1f)) {
            Text(project.title, style = MaterialTheme.typography.titleSmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
            project.sessionName?.let {
                Text(it, style = MaterialTheme.typography.bodySmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
            }
        }
        if (approvals > 0) Badge { Text(approvals.toString()) }
    }
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ProjectTabs(scope: ScreenScope, projection: HostProjection, actions: PairingActions) {
    PrimaryScrollableTabRow(modifier = Modifier.testTag("workspace-tabs"), edgePadding = 0.dp,
        selectedTabIndex = projection.projects.indexOfFirst { it.id == projection.selectedId }.coerceAtLeast(0)) {
        projection.projects.forEach { project -> Tab(selected = project.id == projection.selectedId,
            modifier = Modifier.widthIn(min = 96.dp, max = 220.dp).heightIn(min = 48.dp)
                .testTag("workspace-tab-${project.id}").semantics { contentDescription = project.title },
            enabled = switchable(scope, projection, project),
            onClick = { switchTo(scope, project, actions) },
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
                        Icon(Icons.AutoMirrored.Outlined.CallSplit, contentDescription = "Add Git worktree")
                    }
                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "worktree", onClick = { subtabUi.closeWorktree = subtabs.activeId }) {
                        Icon(Icons.Outlined.DeleteOutline, contentDescription = "Remove Git worktree")
                    }
                }
                if (subtabs.directoryControl) {
                    IconButton(enabled = enabled && subtabs.tabs.size < 32, onClick = { subtabUi.directoryTabDialog = true }) {
                        Icon(Icons.Outlined.CreateNewFolder, contentDescription = "Add directory tab")
                    }
                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "in_place", onClick = { subtabUi.closeDirectoryTab = subtabs.activeId }) {
                        Icon(Icons.Outlined.RemoveCircleOutline, contentDescription = "Close directory tab")
                    }
                }
                IconButton(enabled = enabled, onClick = {
                    subtabUi.renameSubtab = subtabs.activeId; subtabUi.subtabName = subtabs.tabs[index].name
                }) { Icon(Icons.Outlined.Edit, contentDescription = "Rename worktree tab") }
                IconButton(enabled = enabled && index > 1, onClick = {
                    actions.subtab("subtab_move", subtabs.activeId, subtabs.tabs[index - 1].id)
                }) { Icon(Icons.Outlined.KeyboardDoubleArrowLeft, contentDescription = "Move worktree tab left") }
                IconButton(enabled = enabled && index > 0 && index < subtabs.tabs.lastIndex, onClick = {
                    actions.subtab("subtab_move", subtabs.activeId, subtabs.tabs.getOrNull(index + 2)?.id)
                }) { Icon(Icons.Outlined.KeyboardDoubleArrowRight, contentDescription = "Move worktree tab right") }
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
