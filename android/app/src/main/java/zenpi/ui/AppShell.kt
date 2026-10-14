package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.consumeWindowInsets
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.Menu
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.Badge
import androidx.compose.material3.BadgedBox
import androidx.compose.material3.Button
import androidx.compose.material3.DrawerValue
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilledTonalButton
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalDrawerSheet
import androidx.compose.material3.ModalNavigationDrawer
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberDrawerState
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.State
import androidx.compose.runtime.derivedStateOf
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberCoroutineScope
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.ui.unit.dp
import kotlinx.coroutines.launch
import zenpi.pairing.DeviceActionPolicy
import zenpi.pairing.PairingConfirmationPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.SynchronizationStatus
import zenpi.ui.dialogs.DeviceDialogs
import zenpi.ui.dialogs.DiffDialog
import zenpi.ui.dialogs.NewSessionDialog
import zenpi.ui.dialogs.RenameSessionDialog
import zenpi.ui.dialogs.SessionHistoryDialog
import zenpi.ui.dialogs.SessionsDialog
import zenpi.ui.dialogs.SubtabDialogs
import zenpi.ui.dialogs.TerminalHistoryDialog
import zenpi.ui.dialogs.TranscriptDialog
import zenpi.ui.dialogs.WorkerHistoryDialog
import zenpi.ui.dialogs.WorkerStatusesDialog
import zenpi.ui.dialogs.WorkspaceDialogs

/**
 * Root of the paired-device screen: header, workspace, invitation and the modal layer.
 * Project and worktree switching lives in [SwitcherDrawer], hosted in a modal drawer opened
 * from the top bar (badged with pending approvals). [inlineSwitcher] renders the same drawer
 * content inline above the workspace instead; only the [PairingScreen] test adapter uses it.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun AppShell(state: PairingState, actions: PairingActions, inlineSwitcher: Boolean = false) {
    val scope = ScreenScope(state)
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val workerHistoryScroll = rememberScrollState()
    LaunchedEffect(state.workerHistoryRequest, state.workerHistoryPage?.offset) { workerHistoryScroll.scrollTo(0) }
    val workspaceUi = remember(scope.hostScope, scope.sessionScope) { WorkspaceUiState() }
    val subtabUi = remember(scope.hostScope, scope.sessionScope, scope.subtabScope) { SubtabUiState() }
    val directory = rememberSaveable(scope.hostScope, scope.sessionScope) { mutableStateOf("") }
    val revokeDialog = remember(state.host) { mutableStateOf(false) }
    val forgetDialog = remember(state.host, state.resetRequired) { mutableStateOf(false) }
    val message = rememberDraft(scope.draftKey, DraftPolicy.promptMarker(state))
    val steer = rememberDraft(DraftPolicy.steerKey(state), null)
    val workerPrompt = rememberDraft(scope.draftKey, DraftPolicy.workerMarker(state))
    val clock = rememberInviteClock(state.invite)
    // One countdown reading per tick, shared by the phase label and the invitation.
    val remaining = remember(state.invite) { derivedStateOf { state.invite?.remainingSeconds(clock.value) ?: 0L } }
    LaunchedEffect(state.resourcesOpen, state.filesOpen) {
        if (state.resourcesOpen || state.filesOpen) { keyboard?.hide(); focus.clearFocus() }
    }
    val drawer = rememberDrawerState(DrawerValue.Closed)
    val drawerScope = rememberCoroutineScope()
    val switcher = !inlineSwitcher && state.projection != null
    val screen = @Composable {
        Scaffold(contentWindowInsets = WindowInsets.safeDrawing,
            topBar = { TopAppBar(title = { Text("Zenpi") }, windowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal),
                navigationIcon = { if (switcher) SwitcherButton(SwitcherInbox.pendingApprovals(state.projection)) { drawerScope.launch { drawer.open() } } }) }) { padding ->
            Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding().verticalScroll(rememberScrollState()).padding(20.dp),
                verticalArrangement = Arrangement.spacedBy(16.dp)) {
                ConnectionHeader(state, remaining, actions)
                state.projection?.let { projection ->
                    if (projection.projects.isEmpty()) Text("No projects authorized")
                    else {
                        if (inlineSwitcher) SwitcherDrawer(scope, projection, workspaceUi, subtabUi, actions)
                        WorkspaceSection(scope, projection, workspaceUi, subtabUi, message, steer, workerPrompt, actions)
                    }
                }
                state.invite?.let { InviteSection(state, clock, remaining, actions) }
                DeviceSection(state, actions, revokeDialog, forgetDialog)
            }
            if (revokeDialog.value) DeviceDialogs.Revoke(state, revokeDialog, actions)
        }
    }
    if (inlineSwitcher) screen()
    else ModalNavigationDrawer(drawerState = drawer, gesturesEnabled = switcher, drawerContent = {
        ModalDrawerSheet(Modifier.fillMaxWidth(0.85f)) {
            Column(Modifier.verticalScroll(rememberScrollState()).padding(16.dp), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                state.projection?.takeIf { it.projects.isNotEmpty() }?.let { projection ->
                    SwitcherDrawer(scope, projection, workspaceUi, subtabUi, actions) { drawerScope.launch { drawer.close() } }
                }
            }
        }
    }, content = screen)
    if (forgetDialog.value) DeviceDialogs.Forget(state, forgetDialog, actions)
    if (state.diffOpen) DiffDialog(state, scope, actions)
    if (state.workerStatusesOpen) WorkerStatusesDialog(state, actions)
    if (state.sessionHistoryOpen && state.transcriptTurn == null) SessionHistoryDialog(state, actions)
    state.workerHistoryRequest?.takeIf { state.transcriptTurn == null }?.let { WorkerHistoryDialog(state, it, workerHistoryScroll, actions) }
    workspaceUi.newSessionTarget?.let { NewSessionDialog(state, it, workspaceUi, actions) }
    workspaceUi.renameSessionTarget?.let { RenameSessionDialog(state, it, workspaceUi, actions) }
    if (state.sessionsOpen) SessionsDialog(state, scope, actions)
    if (state.terminalOpen) TerminalHistoryDialog(state, actions)
    state.transcriptTurn?.let { TranscriptDialog(state, it, actions) }
    workspaceUi.approvalDecision?.let { (target, decision) -> ApprovalDecisionDialog(state, target, decision, workspaceUi, actions) }
    WorkspaceDialogs(state, scope, workspaceUi, directory, actions)
    SubtabDialogs(state, scope, subtabUi, actions)
}

/** Opens the switcher drawer; the badge counts approvals waiting across all projects. */
@Composable
private fun SwitcherButton(approvals: Int, onClick: () -> Unit) {
    IconButton(onClick = onClick, modifier = Modifier.testTag("switcher-open")) {
        BadgedBox(badge = { if (approvals > 0) Badge { Text(approvals.toString()) } }) {
            Icon(Icons.Outlined.Menu, contentDescription = when (approvals) { 0 -> "Open switcher"; 1 -> "Open switcher, 1 pending approval"
                else -> "Open switcher, $approvals pending approvals" })
        }
    }
}

/** Connection phase, scan entry and the paired host card. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ConnectionHeader(state: PairingState, remaining: State<Long>, actions: ConnectionActions) {
    Text("Desktop", style = MaterialTheme.typography.headlineSmall)
    PhaseLabel(state, remaining)
    SynchronizationStatus.label(state)?.let {
        Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.testTag("synchronization-status"))
    }
    if (state.canScan) {
        FilledTonalButton(onClick = actions::scan) {
            Icon(Icons.Outlined.QrCodeScanner, contentDescription = null, modifier = Modifier.size(20.dp))
            Spacer(Modifier.width(8.dp)); Text("Scan QR")
        }
    }
    state.scanError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    state.host?.let { host ->
        val clipboard = LocalClipboardManager.current
        if (state.invite == null) Text(if (host.host.contains(':')) "[${host.host}]:${host.port}" else "${host.host}:${host.port}", style = MaterialTheme.typography.titleMedium)
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Column(Modifier.weight(1f)) {
                Text("Device ID", style = MaterialTheme.typography.labelMedium)
                Text(host.deviceId.chunked(16).joinToString("\n"), style = MaterialTheme.typography.bodySmall,
                    fontFamily = FontFamily.Monospace, modifier = Modifier.testTag("paired-device-id"))
            }
            TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                tooltip = { PlainTooltip { Text("Copy device ID") } }, state = rememberTooltipState()) {
                IconButton(onClick = { clipboard.setText(AnnotatedString(host.deviceId)) }, enabled = !state.resetting) {
                    Icon(Icons.Outlined.ContentCopy, contentDescription = "Copy device ID")
                }
            }
        }
        Button(onClick = { actions.refresh(if (state.selectedProjectUnavailable) null else state.projection?.selectedId) },
            enabled = !state.scanInProgress && state.phase != PairingPhase.CONNECTING && state.pendingRevoke == null) { Text("Refresh host") }
    }
}

/** Only this label (and the invitation) read the countdown clock. */
@Composable
private fun PhaseLabel(state: PairingState, remaining: State<Long>) {
    Text(if (state.resetting) "Removing local device" else when (state.phase) {
        PairingPhase.OFFLINE -> "Offline"
        PairingPhase.CONFIRMING -> if (remaining.value == 0L) "Invitation expired" else "Confirm host"
        PairingPhase.CONNECTING -> "Connecting"
        PairingPhase.ACCEPTED -> "Invitation accepted"
        PairingPhase.CONNECTED -> "Connected"
        PairingPhase.ERROR -> "Connection unavailable"
    }, color = if (state.phase == PairingPhase.ERROR) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary)
}

@Composable
private fun InviteSection(state: PairingState, clock: State<Long>, remaining: State<Long>, actions: ConnectionActions) {
    val invite = state.invite ?: return
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    var code by remember(state.invite) { mutableStateOf("") }
    HorizontalDivider()
    Text(invite.address, style = MaterialTheme.typography.titleMedium)
    Text("Host SHA-256", style = MaterialTheme.typography.labelMedium)
    Text(invite.fingerprint.chunked(16).joinToString("\n"), style = MaterialTheme.typography.bodySmall)
    if (state.phase == PairingPhase.CONFIRMING) {
        val now = clock.value
        val left = remaining.value
        Text("Expires in ${left}s", style = MaterialTheme.typography.bodySmall)
        OutlinedTextField(value = code, onValueChange = { code = it.filter { digit -> digit in '0'..'9' }.take(6) },
            label = { Text("Desktop safety code") }, singleLine = true,
            enabled = state.canScan && left > 0,
            keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
            keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() }), modifier = Modifier.fillMaxWidth())
        Button(onClick = { actions.confirm(code) }, enabled = PairingConfirmationPolicy.allows(state, code, now)) { Text("Confirm host") }
    }
}

/** Registration status, device lifecycle buttons, errors and request retry. */
@Composable
private fun DeviceSection(state: PairingState, actions: ConnectionActions, revokeDialog: MutableState<Boolean>, forgetDialog: MutableState<Boolean>) {
    if (state.phase == PairingPhase.ACCEPTED && state.registered && state.host != null) {
        Text("Device registered", style = MaterialTheme.typography.titleSmall)
        Text("Host control unavailable", color = MaterialTheme.colorScheme.secondary)
    }
    if (state.registered) {
        if (state.pendingRevoke != null) TextButton(onClick = actions::checkRevoke, enabled = DeviceActionPolicy.checkRevocation(state)) { Text("Check revocation status") }
        TextButton(onClick = { revokeDialog.value = true }, enabled = DeviceActionPolicy.revoke(state)) { Text("Revoke device on host") }
        TextButton(onClick = { forgetDialog.value = true }, enabled = DeviceActionPolicy.forget(state)) { Text("Forget device locally") }
    }
    if (state.resetRequired && !state.resetting) TextButton(onClick = { forgetDialog.value = true }, enabled = DeviceActionPolicy.forget(state)) { Text("Reset local device") }
    state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
    if (state.workspaceFailureCode == "unknown_outcome") {
        Text("Desktop could not confirm the write. Retry the same request only after checking its result.",
            color = MaterialTheme.colorScheme.error,
            modifier = Modifier.testTag("workspace-unknown-outcome"))
    }
    if (state.pendingWorkspace != null && state.phase != PairingPhase.CONNECTING) {
        Button(onClick = actions::retryWorkspace) { Icon(Icons.Outlined.Refresh, null); Spacer(Modifier.width(8.dp)); Text("Retry same request") }
    }
    if (state.phase == PairingPhase.CONNECTING) LinearProgressIndicator(Modifier.fillMaxWidth())
    if (state.phase != PairingPhase.OFFLINE && !state.resetting && !state.resetRequired) TextButton(onClick = actions::cancel) { Text("Cancel") }
}
