package zenpi

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.result.contract.ActivityResultContracts
import androidx.activity.viewModels
import androidx.compose.foundation.layout.*
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.QrCodeScanner
import androidx.compose.material.icons.outlined.Add
import androidx.compose.material.icons.outlined.Remove
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material.icons.outlined.Edit
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Send
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.ArrowDropDown
import androidx.compose.material.icons.outlined.AccountTree
import androidx.compose.material.icons.outlined.DeleteOutline
import androidx.compose.material.icons.outlined.Difference
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.InsertDriveFile
import androidx.compose.material.icons.outlined.Tune
import androidx.compose.material.icons.outlined.ContentCopy
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.KeyboardReturn
import androidx.compose.material.icons.outlined.KeyboardTab
import androidx.compose.material.icons.outlined.Keyboard
import androidx.compose.material.icons.outlined.Terminal
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.outlined.Search
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material3.*
import androidx.compose.runtime.*
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalClipboardManager
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.input.KeyboardType
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.ui.unit.dp
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import zenpi.pairing.*
import kotlinx.coroutines.delay

class MainActivity : ComponentActivity() {
    private val model: PairingViewModel by viewModels()
    private val network by lazy { zenpi.lifecycle.NetworkMonitor(this, model::networkChanged) }
    private val scanner = registerForActivityResult(ScanContract()) { result ->
        model.scanResult(result.contents,
            missingPermission = result.originalIntent?.getBooleanExtra("MISSING_CAMERA_PERMISSION", false) == true,
            timedOut = result.originalIntent?.getBooleanExtra("TIMEOUT", false) == true)
    }
    private fun takePairingIntent(incoming: Intent): String? {
        val uri = incoming.dataString
        // Keep the task's intent usable without retaining its one-use QR credential.
        setIntent(Intent(incoming).apply { setDataAndType(null, incoming.type) })
        return uri
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        val uri = takePairingIntent(intent)
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) uri?.let(model::acceptUri)
        setContent {
            MaterialTheme(colorScheme = lightColorScheme(primary = Color(0xFF166A4C), secondary = Color(0xFF934A25),
                background = Color(0xFFF5F7F8), surface = Color.White)) {
                val state by model.state.collectAsState()
                PairingScreen(state, onScan = {
                    if (model.beginScan()) {
                        if (!packageManager.hasSystemFeature(android.content.pm.PackageManager.FEATURE_CAMERA_ANY)) model.cameraUnavailable()
                        else try {
                            scanner.launch(ScanOptions().setCaptureActivity(PairingCaptureActivity::class.java).setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                                .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false)
                                .setTimeout(120_000).addExtra("SHOW_MISSING_CAMERA_PERMISSION_DIALOG", false).setPrompt("Zenpi Desktop"))
                        } catch (_: android.content.ActivityNotFoundException) { model.cameraUnavailable() }
                    }
                }, onConfirm = model::confirm, onCancel = model::cancel, onForget = model::forget, onRefresh = { model.refresh(it) },
                    onWorkspace = model::workspace, onRetryWorkspace = model::retryWorkspace, onSubtab = model::subtab, onAddDirectoryTab = model::addDirectoryTab, onAddWorktreeTab = model::addWorktreeTab, onRevoke = model::revokeDevice, onCheckRevoke = model::checkRevocation, onPrompt = model::prompt, onCancelPrompt = model::cancelPrompt, onSteer = model::steer, onApprove = model::approve,
                    onDiff = model::reviewDiff, onCloseDiff = model::closeDiff, onNextDiff = model::nextDiff, onPreviousDiff = model::previousDiff, onTranscript = model::readTranscript, onCloseTranscript = model::dismissTranscript,
                    onPreviousTranscript = model::previousTranscript,
                    onWorkerTranscript = model::readWorkerTranscript,
                    onWorkerHistory = model::readWorkerHistory, onCloseWorkerHistory = model::closeWorkerHistory,
                    onWorkerHistoryTurn = model::readWorkerHistoryTurn,
                    onWorkerToolArguments = model::readWorkerToolArguments,
                    onToolArguments = model::readToolArguments,
                    onNewSession = model::newSession, onRenameSession = model::renameSession,
                    onSessions = { model.readSessions(it) }, onSearchSessions = model::searchSessions, onCloseSessions = model::closeSessions, onResumeSession = model::resumeSession,
                    onSessionHistory = model::readSessionHistory, onPreviousSessionHistory = model::previousSessionHistory,
                    onCloseSessionHistory = model::closeSessionHistory, onSessionHistoryTurn = { model.readSessionHistoryTurn(it) },
                    onSessionHistoryTool = { id, call -> model.readSessionHistoryTurn(id, call) },
                    onTerminalInput = model::terminalInput, onTerminalResize = model::terminalResize,
                    onOpenDesktopTerminal = model::openDesktopTerminal,
                    onTerminalPage = model::readTerminal, onPreviousTerminalPage = model::previousTerminal, onCloseTerminal = model::closeTerminal,
                    onPreviousWorkerHistory = model::previousWorkerHistory,
                    onWorkerStatuses = model::readWorkerStatuses, onCloseWorkerStatuses = model::closeWorkerStatuses,
                    onFiles = model::browseFiles, onCloseFiles = model::closeFiles,
                    onFilePage = model::browseFilePage,
                    onResources = model::browseResources, onCloseResources = model::closeResources, onReloadResources = model::reloadResources,
                    onModel = {model.inspectModel()}, onCloseModel = model::closeModel, onSelectModel = model::selectModel,
                    onModelPage = model::inspectModel,
                    onSelectReasoning = model::selectReasoning, onWorkerPrompt = model::workerPrompt, onCancelWorker = model::cancelWorker, onConcurrency = model::concurrency)
            }
        }
    }
    override fun onNewIntent(intent: Intent) {
        val uri = takePairingIntent(intent)
        super.onNewIntent(getIntent())
        uri?.let(model::acceptUri)
    }
    override fun onStart() { super.onStart(); zenpi.lifecycle.ResumeScheduler.cancel(this); network.start(); model.setForeground(true) }
    override fun onStop() { network.stop(); model.setForeground(false); zenpi.lifecycle.ResumeScheduler.enqueue(this); super.onStop() }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
fun PairingScreen(state: PairingState, onScan: () -> Unit, onConfirm: (String) -> Unit, onCancel: () -> Unit, onForget: () -> Unit = {}, onRefresh: (String?) -> Unit = {},
    onWorkspace: (String, String, String?) -> Unit = { _, _, _ -> }, onRetryWorkspace: () -> Unit = {},
    onSubtab: (String, String, String?) -> Unit = { _, _, _ -> }, onAddDirectoryTab: (String, String) -> Unit = { _, _ -> }, onRevoke: () -> Unit = {}, onCheckRevoke: () -> Unit = {}, onPrompt: (String) -> Unit = {}, onCancelPrompt: (String) -> Unit = {},
    onApprove: (zenpi.transport.RemoteApproval, String) -> Unit = { _, _ -> }, onSteer: (String, String) -> Unit = { _, _ -> },
    onAddWorktreeTab: (String) -> Unit = {}, onDiff: (String) -> Unit = {}, onCloseDiff: () -> Unit = {},
    onNextDiff: () -> Unit = {}, onPreviousDiff: () -> Unit = {},
    onTranscript: (String, Boolean) -> Unit = { _, _ -> }, onCloseTranscript: () -> Unit = {},
    onWorkerTranscript: (String, Boolean) -> Unit = { _, _ -> },
    onWorkerHistory: (String, Boolean) -> Unit = { _, _ -> }, onCloseWorkerHistory: () -> Unit = {},
    onWorkerHistoryTurn: (String) -> Unit = {},
    onWorkerToolArguments: (String, String) -> Unit = { _, _ -> },
    onToolArguments: (String, String) -> Unit = { _, _ -> },
    onNewSession: () -> Unit = {},
    onRenameSession: (String) -> Unit = {},
    onSessions: (Int) -> Unit = {}, onCloseSessions: () -> Unit = {}, onResumeSession: (String) -> Unit = {},
    onSearchSessions: (String) -> Unit = {},
    onSessionHistory: (Boolean) -> Unit = {}, onPreviousSessionHistory: () -> Unit = {}, onCloseSessionHistory: () -> Unit = {},
    onSessionHistoryTurn: (String) -> Unit = {}, onSessionHistoryTool: (String, String) -> Unit = { _, _ -> },
    onTerminalInput: (String) -> Unit = {}, onTerminalResize: (Int, Int) -> Unit = { _, _ -> },
    onOpenDesktopTerminal: () -> Unit = {},
    onTerminalPage: (Boolean) -> Unit = {}, onPreviousTerminalPage: () -> Unit = {}, onCloseTerminal: () -> Unit = {},
    onPreviousWorkerHistory: () -> Unit = {},
    onWorkerStatuses: (Int) -> Unit = {}, onCloseWorkerStatuses: () -> Unit = {},
    onFiles: (String) -> Unit = {}, onCloseFiles: () -> Unit = {},
    onFilePage: (Int) -> Unit = {},
    onResources: (Int) -> Unit = {}, onCloseResources: () -> Unit = {}, onReloadResources: () -> Unit = {},
    onModel: () -> Unit = {}, onCloseModel: () -> Unit = {}, onSelectModel: (String) -> Unit = {}, onSelectReasoning: (String?) -> Unit = {}, onModelPage: (Int) -> Unit = {},
    onPreviousTranscript: () -> Unit = {}, onWorkerPrompt: (String) -> Unit = {}, onCancelWorker: (String) -> Unit = {}, onConcurrency: (String, Int) -> Unit = { _, _ -> }) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val workerHistoryScroll = rememberScrollState()
    LaunchedEffect(state.workerHistoryRequest, state.workerHistoryPage?.offset) { workerHistoryScroll.scrollTo(0) }
    var code by remember(state.invite) { mutableStateOf("") }
    val hostScope = state.host?.let { it.fingerprint to it.deviceId }
    val sessionScope = state.projection?.let { it.selectedId to it.sessionId }
    val subtabScope = state.projection?.subtabs?.let { it.activeId to it.revision }
    val promptTarget = state.projection?.projects?.singleOrNull {
        it.id == state.projection.selectedId && it.sessionId == state.projection.sessionId
    }?.promptStatuses?.firstOrNull { it.state in setOf("accepted", "running") }?.requestId
    val mutationEnabled = state.phase == PairingPhase.CONNECTED && !state.scanInProgress && state.pendingWorkspace == null && state.pendingRevoke == null
        && !state.resetting && !state.resetRequired
    val workspaceEnabled = mutationEnabled && state.projection?.projects?.singleOrNull { it.id == state.projection.selectedId }?.workspaceControl == true
    val currentSubtabs = zenpi.transport.SubtabProjectionCodec.current(state.projection)
    val subtabEnabled = workspaceEnabled && currentSubtabs?.control == true
    var openDialog by remember(hostScope, sessionScope) { mutableStateOf(false) }
    var closeProject by remember(hostScope, sessionScope) { mutableStateOf<String?>(null) }
    var newSessionTarget by remember(hostScope, sessionScope) { mutableStateOf<Pair<String, String>?>(null) }
    var renameSessionTarget by remember(hostScope, sessionScope) { mutableStateOf<Pair<String, String>?>(null) }
    var sessionNameDraft by remember(hostScope, sessionScope) { mutableStateOf("") }
    var resumeTarget by remember(hostScope, sessionScope, state.sessionsOpen, state.sessions?.snapshotId, state.sessions?.offset, state.sessions?.query) { mutableStateOf<String?>(null) }
    var sessionSearch by remember(hostScope, sessionScope, state.sessionsOpen) { mutableStateOf(state.sessionsQuery.orEmpty()) }
    var directory by rememberSaveable(hostScope, sessionScope) { mutableStateOf("") }
    var renameSubtab by remember(hostScope, sessionScope, subtabScope) { mutableStateOf<String?>(null) }
    var subtabName by remember(hostScope, sessionScope, subtabScope) { mutableStateOf("") }
    var directoryTabDialog by remember(hostScope, sessionScope, subtabScope) { mutableStateOf(false) }
    var directoryTabName by remember(hostScope, sessionScope, subtabScope) { mutableStateOf("") }
    var directoryTabPath by remember(hostScope, sessionScope, subtabScope) { mutableStateOf("") }
    var closeDirectoryTab by remember(hostScope, sessionScope, subtabScope) { mutableStateOf<String?>(null) }
    var worktreeDialog by remember(hostScope, sessionScope, subtabScope) { mutableStateOf(false) }
    var worktreeName by remember(hostScope, sessionScope, subtabScope) { mutableStateOf("") }
    var closeWorktree by remember(hostScope, sessionScope, subtabScope) { mutableStateOf<String?>(null) }
    var revokeDialog by remember(state.host) { mutableStateOf(false) }
    var forgetDialog by remember(state.host, state.resetRequired) { mutableStateOf(false) }
    var approvalDecision by remember(hostScope, sessionScope) { mutableStateOf<Pair<zenpi.transport.RemoteApproval, String>?>(null) }
    var prompt by rememberSaveable(hostScope, sessionScope, subtabScope, promptTarget) { mutableStateOf("") }
    var workerPrompt by rememberSaveable(hostScope, sessionScope, subtabScope) { mutableStateOf("") }
    var modelInput by remember(hostScope, state.modelStatus?.projectId, state.modelStatus?.sessionId, state.modelStatus?.selectionRevision, state.modelOpen) {
        mutableStateOf(state.modelStatus?.model.orEmpty())
    }
    var reasoningMenu by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(false) }
    var modelMenu by remember(state.modelOpen,state.modelStatus?.catalog?.snapshotId) {mutableStateOf(false)}
    var reasoningInput by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(state.modelStatus?.reasoningEffort) }
    var now by remember { mutableLongStateOf(System.currentTimeMillis() / 1000) }
    var clockTick by remember { mutableLongStateOf(0) }
    val clipboard = LocalClipboardManager.current
    LaunchedEffect(state.resourcesOpen, state.filesOpen) {
        if (state.resourcesOpen || state.filesOpen) { keyboard?.hide(); focus.clearFocus() }
    }
    LaunchedEffect(state.invite) { while (true) { now = System.currentTimeMillis() / 1000; clockTick++; delay(1000) } }
    val remaining = clockTick.let { state.invite?.remainingSeconds(now) ?: 0 }
    Scaffold(contentWindowInsets = WindowInsets.safeDrawing,
        topBar = { TopAppBar(title = { Text("Zenpi") }, windowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal)) }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).consumeWindowInsets(padding).imePadding().verticalScroll(rememberScrollState()).padding(20.dp),
            verticalArrangement = Arrangement.spacedBy(16.dp)) {
            Text("Desktop", style = MaterialTheme.typography.headlineSmall)
            Text(if (state.resetting) "Removing local device" else when(state.phase) {
                PairingPhase.OFFLINE -> "Offline"
                PairingPhase.CONFIRMING -> if (remaining == 0L) "Invitation expired" else "Confirm host"
                PairingPhase.CONNECTING -> "Connecting"
                PairingPhase.ACCEPTED -> "Invitation accepted"
                PairingPhase.CONNECTED -> "Connected"
                PairingPhase.ERROR -> "Connection unavailable"
            }, color = if (state.phase == PairingPhase.ERROR) MaterialTheme.colorScheme.error else MaterialTheme.colorScheme.primary)
            SynchronizationStatus.label(state)?.let {
                Text(it, style = MaterialTheme.typography.bodyMedium, modifier = Modifier.testTag("synchronization-status"))
            }
            if (state.canScan) {
                FilledTonalButton(onClick = onScan) {
                    Icon(Icons.Outlined.QrCodeScanner, contentDescription = null, modifier = Modifier.size(20.dp))
                    Spacer(Modifier.width(8.dp)); Text("Scan QR")
                }
            }
            state.scanError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.host?.let { host ->
                if (state.invite == null) Text(if(host.host.contains(':')) "[${host.host}]:${host.port}" else "${host.host}:${host.port}", style = MaterialTheme.typography.titleMedium)
                Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
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
                Button(onClick = { onRefresh(if (state.selectedProjectUnavailable) null else state.projection?.selectedId) }, enabled = !state.scanInProgress && state.phase != PairingPhase.CONNECTING && state.pendingRevoke == null) { Text("Refresh host") }
            }
            state.projection?.let { projection ->
                if (projection.projects.isEmpty()) Text("No projects authorized")
                else {
                    PrimaryScrollableTabRow(modifier = Modifier.testTag("workspace-tabs"), edgePadding = 0.dp,
                        selectedTabIndex = projection.projects.indexOfFirst { it.id == projection.selectedId }.coerceAtLeast(0)) {
                        projection.projects.forEach { project -> Tab(selected = project.id == projection.selectedId,
                            modifier = Modifier.widthIn(min = 96.dp, max = 220.dp).heightIn(min = 48.dp)
                                .testTag("workspace-tab-${project.id}").semantics { contentDescription = project.title },
                            enabled = !state.scanInProgress && !state.resetting && !state.resetRequired
                                && state.phase != PairingPhase.CONNECTING && state.pendingWorkspace == null && state.pendingRevoke == null
                                && projection.projects.singleOrNull { it.id == project.id } != null,
                            onClick = { if (project.workspaceControl && state.phase == PairingPhase.CONNECTED) onWorkspace("workspace_select", project.id, null) else onRefresh(project.id) },
                            text = { Text(project.title, maxLines = 2, overflow = TextOverflow.Ellipsis) }) }
                    }
                    val selected = projection.projects.singleOrNull { it.id == projection.selectedId }
                    selected?.sessionName?.let { Text(it, style = MaterialTheme.typography.titleSmall, maxLines = 2, overflow = TextOverflow.Ellipsis) }
                    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Sessions") } }, state = rememberTooltipState()) {
                        IconButton(onClick = { onSessions(-1) }, enabled = SessionCatalogReadPolicy.browse(state)) { Icon(Icons.Outlined.History, contentDescription = "Browse sessions") }
                    }
                    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                        tooltip = { PlainTooltip { Text("Browse conversation") } }, state = rememberTooltipState()) {
                        IconButton(onClick = { onSessionHistory(false) }, enabled = SessionHistoryReadPolicy.browse(state)) {
                            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse conversation")
                        }
                    }
                    if (selected?.sessionControl == true && zenpi.transport.SessionCommandCodec.mainSelected(projection)) {
                        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Rename session") } }, state = rememberTooltipState()) {
                            IconButton(onClick = { renameSessionTarget = selected.id to selected.sessionId; sessionNameDraft = selected.sessionName.orEmpty() },
                                enabled = state.phase == PairingPhase.CONNECTED && !state.scanInProgress && !state.resetting && !state.resetRequired
                                    && zenpi.transport.SessionCatalogCodec.readable(projection)
                                    && state.pendingWorkspace == null && state.pendingRevoke == null && projection.phase == "idle") {
                                Icon(Icons.Outlined.Edit, contentDescription = "Rename session")
                            }
                        }
                        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("New session") } }, state = rememberTooltipState()) {
                            IconButton(onClick = { newSessionTarget = selected.id to selected.sessionId }, enabled = state.phase == PairingPhase.CONNECTED
                                && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(projection)
                                && state.pendingWorkspace == null && state.pendingRevoke == null && projection.phase == "idle"
                                && selected.promptStatuses.none { it.state in setOf("accepted", "running") }) {
                                Icon(Icons.Outlined.Add, contentDescription = "New session")
                            }
                        }
                    }
                    if (selected?.diffAvailable == true) {
                        IconButton(onClick = { onDiff(".") }, enabled = DiffReadPolicy.browse(state)) {
                            Icon(Icons.Outlined.Difference, contentDescription = "Review diff")
                        }
                    }
                    if (selected != null && (!selected.promptControl || selected.promptStatuses.any { it.state in setOf("accepted", "running") })) {
                        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                            tooltip = { PlainTooltip { Text("Browse host resources") } }, state = rememberTooltipState()) {
                            IconButton(onClick = { onResources(-1) }, enabled = ResourceReadPolicy.browse(state)) {
                                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse host resources")
                            }
                        }
                    }
                    if (selected != null && (!selected.promptControl || selected.promptStatuses.any { it.state in setOf("accepted", "running") })
                        && projection.subtabs != null) TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                        tooltip = { PlainTooltip { Text("Browse host files") } }, state = rememberTooltipState()) {
                        IconButton(onClick = { onFiles(".") }, enabled = FileReadPolicy.browse(state)) {
                            Icon(Icons.Outlined.Folder, contentDescription = "Browse host files")
                        }
                    }
                    projection.subtabs?.takeIf { it.tabs.isNotEmpty() }?.let { subtabs ->
                        SecondaryScrollableTabRow(modifier = Modifier.testTag("secondary-tabs"), edgePadding = 0.dp,
                            selectedTabIndex = subtabs.tabs.indexOfFirst { it.id == subtabs.activeId }.coerceAtLeast(0)) {
                            subtabs.tabs.forEach { tab -> Tab(selected = tab.id == subtabs.activeId,
                                modifier = Modifier.widthIn(min = 96.dp, max = 220.dp).heightIn(min = 48.dp)
                                    .testTag("subtab-${tab.id}").semantics { contentDescription = tab.name },
                                enabled = subtabEnabled && subtabs.tabs.singleOrNull { it.id == tab.id } != null,
                                onClick = { onSubtab("subtab_select", tab.id, null) }, text = { Text(tab.name, maxLines = 2, overflow = TextOverflow.Ellipsis) }) }
                        }
                        if (subtabs.control && currentSubtabs != null) {
                            val index = subtabs.tabs.indexOfFirst { it.id == subtabs.activeId }
                            val enabled = subtabEnabled
                            androidx.compose.foundation.layout.FlowRow {
                                if (subtabs.worktreeControl) {
                                    IconButton(enabled = enabled && subtabs.tabs.size < 32, onClick = { worktreeDialog = true }) {
                                        Icon(Icons.Outlined.AccountTree, contentDescription = "Add Git worktree")
                                    }
                                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "worktree", onClick = { closeWorktree = subtabs.activeId }) {
                                        Icon(Icons.Outlined.DeleteOutline, contentDescription = "Remove Git worktree")
                                    }
                                }
                                if (subtabs.directoryControl) {
                                    IconButton(enabled = enabled && subtabs.tabs.size < 32, onClick = { directoryTabDialog = true }) {
                                        Icon(Icons.Outlined.Add, contentDescription = "Add directory tab")
                                    }
                                    IconButton(enabled = enabled && subtabs.tabs[index].kind == "in_place", onClick = { closeDirectoryTab = subtabs.activeId }) {
                                        Icon(Icons.Outlined.Close, contentDescription = "Close directory tab")
                                    }
                                }
                                IconButton(enabled = enabled, onClick = {
                                    renameSubtab = subtabs.activeId; subtabName = subtabs.tabs[index].name
                                }) { Icon(Icons.Outlined.Edit, contentDescription = "Rename worktree tab") }
                                IconButton(enabled = enabled && index > 1, onClick = {
                                    onSubtab("subtab_move", subtabs.activeId, subtabs.tabs[index - 1].id)
                                }) { Icon(Icons.Outlined.ArrowBack, contentDescription = "Move worktree tab left") }
                                IconButton(enabled = enabled && index > 0 && index < subtabs.tabs.lastIndex, onClick = {
                                    onSubtab("subtab_move", subtabs.activeId, subtabs.tabs.getOrNull(index + 2)?.id)
                                }) { Icon(Icons.Outlined.ArrowForward, contentDescription = "Move worktree tab right") }
                            }
                        }
                        if (subtabs.concurrencyControl && currentSubtabs != null) {
                            val tab = currentSubtabs.tabs.single { it.id == currentSubtabs.activeId }
                            val enabled = workspaceEnabled
                            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                Text("Concurrency", modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
                                IconButton(onClick = { onConcurrency(tab.id, tab.concurrency - 1) }, enabled = enabled && tab.concurrency > 1) {
                                    Icon(Icons.Outlined.Remove, contentDescription = "Decrease worktree concurrency")
                                }
                                Text(tab.concurrency.toString(), modifier = Modifier.width(32.dp), style = MaterialTheme.typography.labelLarge,
                                    textAlign = androidx.compose.ui.text.style.TextAlign.Center)
                                IconButton(onClick = { onConcurrency(tab.id, tab.concurrency + 1) }, enabled = enabled && tab.concurrency < 64) {
                                    Icon(Icons.Outlined.Add, contentDescription = "Increase worktree concurrency")
                                }
                            }
                        }
                    }
                    if (selected?.workspaceControl == true) {
                        Row {
                            IconButton(onClick = { openDialog = true }, enabled = workspaceEnabled) {
                                Icon(Icons.Outlined.Add, contentDescription = "Open workspace")
                            }
                            IconButton(onClick = { closeProject = selected.id }, enabled = workspaceEnabled && projection.projects.size > 1) {
                                Icon(Icons.Outlined.Close, contentDescription = "Close workspace")
                            }
                        }
                    }
                    Text(projection.phase ?: "Unavailable", style = MaterialTheme.typography.labelLarge)
                    selected?.pendingApprovals?.takeIf { state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR) }?.forEach { approval ->
                        Column(Modifier.fillMaxWidth().padding(vertical = 8.dp)) {
                            Text("Awaiting approval", style = MaterialTheme.typography.labelLarge)
                            Text(approval.tool, style = MaterialTheme.typography.bodyMedium)
                            Text(when (approval.sideEffect) { "workspace_write" -> "Workspace write"
                                "command_execution" -> "Command execution"; else -> "Read only" }, style = MaterialTheme.typography.labelMedium)
                            Text(approval.requestId, style = MaterialTheme.typography.labelSmall)
                            approval.workerSessionId?.let { Text("Worker $it", style = MaterialTheme.typography.labelSmall) }
                            if (selected.approvalControl) {
                                Row {
                                    val enabled = ApprovalDecisionPolicy.allows(state, approval)
                                    IconButton(onClick = { approvalDecision = approval to "allow" }, enabled = enabled) {
                                        Icon(Icons.Outlined.Check, contentDescription = "Allow approval ${approval.requestId}" + (approval.workerSessionId?.let { " for worker $it" } ?: ""))
                                    }
                                    IconButton(onClick = { approvalDecision = approval to "deny" }, enabled = enabled) {
                                        Icon(Icons.Outlined.Close, contentDescription = "Deny approval ${approval.requestId}" + (approval.workerSessionId?.let { " for worker $it" } ?: ""))
                                    }
                                }
                            }
                        }
                    }
                    projection.projects.find { it.id == projection.selectedId }?.promptStatuses?.forEach { status ->
                        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                            Text(status.requestId.take(8), style = MaterialTheme.typography.labelSmall)
                            Text(when (status.state) { "accepted" -> "Message accepted"; "running" -> "Message running"
                                "succeeded" -> "Message succeeded"; "failed" -> when(status.failureCode){
                                    "invalid_request"->"Message input rejected";"busy"->"Message owner busy";"resource_limit"->"Message limit reached"
                                    "unauthorized","revoked_device"->"Message access denied";else->"Message failed"}; "cancelled" -> "Message cancelled"
                                else -> "Message result unknown" }, style = MaterialTheme.typography.labelMedium,
                                modifier=Modifier.weight(1f).padding(horizontal=8.dp),maxLines=2,overflow=TextOverflow.Ellipsis)
                            if (status.state in setOf("accepted", "running") && selected?.promptControl == true) {
                                IconButton(onClick = { onCancelPrompt(status.requestId) }, enabled = PromptControlPolicy.active(state, status.requestId)) {
                                    Icon(Icons.Outlined.Stop, contentDescription = "Cancel message ${status.requestId}")
                                }
                            }
                        }
                    }
                    Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.End) {
                        IconButton(onClick = { onWorkerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
                            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Browse worktree tasks")
                        }
                        IconButton(onClick = onModel, enabled = ModelReadPolicy.browse(state)) {
                            Icon(Icons.Outlined.Tune, contentDescription = "Inspect host model")
                        }
                    }
                    selected?.workerStatuses?.forEach { status ->
                        Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
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
                            if (status.state == "succeeded") IconButton(onClick = { onWorkerTranscript(status.requestId, false) },
                                enabled = TranscriptReadPolicy.browse(state) && WorkerReadPolicy.projection(state, status.requestId) != null) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read task output ${status.requestId}")
                            }
                            if (status.state == "succeeded") IconButton(onClick = { onWorkerHistory(status.requestId, false) },
                                enabled = !state.workerHistoryLoading && WorkerReadPolicy.projection(state, status.requestId) != null) {
                                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Read task history ${status.requestId}")
                            }
                            if (status.canCancel && selected.workerControl) IconButton(
                                onClick = { onCancelWorker(status.requestId) }, enabled = WorkerTargetPolicy.cancellable(state, status.requestId) != null) {
                                Icon(Icons.Outlined.Stop, contentDescription = "Cancel worktree task ${status.requestId}")
                            }
                        }
                    }
                    if (state.modelOpen) AlertDialog(onDismissRequest = onCloseModel, title = { Text("Host model") },
                        text = { Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(rememberScrollState())) {
                            if (state.modelLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                            state.modelError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                            state.modelStatus?.let { status ->
                                val selectionEnabled = ModelReadPolicy.browse(state) && state.modelError == null
                                    && zenpi.transport.ModelStatusCodec.selectionCurrent(status, projection)
                                    && zenpi.transport.ModelStatusCodec.validChain(state.modelPrevious, status)
                                Text(status.model ?: "Unknown model", style = MaterialTheme.typography.titleMedium, modifier = Modifier.fillMaxWidth(), maxLines = 3, overflow = TextOverflow.Ellipsis)
                                Text("Reasoning: ${status.reasoningEffort ?: "Default"}")
                                if (status.registryBound && selected?.reasoningControl == true && status.reasoningChoices.isNotEmpty()) {
                                    val enabled = selectionEnabled
                                    Box {
                                        OutlinedButton(onClick = { reasoningMenu = true }, enabled = enabled) {
                                            Text(reasoningInput ?: "Provider default"); Icon(Icons.Outlined.ArrowDropDown, contentDescription = "Select reasoning level")
                                        }
                                        DropdownMenu(expanded = reasoningMenu && enabled, onDismissRequest = { reasoningMenu = false }) {
                                            status.reasoningChoices.forEach { effort -> DropdownMenuItem(
                                                enabled = zenpi.transport.ModelStatusCodec.reasoningCurrent(status, projection, effort),
                                                text = { Text(effort ?: "Provider default") },
                                                onClick = { reasoningInput = effort; reasoningMenu = false }) }
                                        }
                                    }
                                    Button(onClick = { onSelectReasoning(reasoningInput) }, enabled = enabled && reasoningInput != status.reasoningEffort
                                        && zenpi.transport.ModelStatusCodec.reasoningCurrent(status, projection, reasoningInput)) {
                                        Icon(Icons.Outlined.Check, contentDescription = null); Spacer(Modifier.width(8.dp)); Text("Apply reasoning")
                                    }
                                }
                                Text("Context budget: ${status.maxTokens}")
                                Text("Reserved output: ${status.reservedOutputTokens}")
                                Text(if (status.registryBound) "Registered model" else "Unregistered model", style = MaterialTheme.typography.labelMedium)
                                if (status.registryBound && selected?.modelControl == true) {
                                    val enabled = selectionEnabled
                                    OutlinedTextField(value = modelInput, onValueChange = { modelInput = it }, label = { Text("Model ID") },
                                        enabled = enabled, singleLine = true, modifier = Modifier.fillMaxWidth())
                                    Button(onClick = { onSelectModel(modelInput) }, enabled = enabled && modelInput.isNotBlank()
                                        && modelInput != status.model && modelInput.toByteArray().size <= 128) {
                                        Icon(Icons.Outlined.Check, contentDescription = null); Spacer(Modifier.width(8.dp)); Text("Apply model")
                                    }
                                }
                                    status.catalog?.let { catalog ->
                                        val enabled = ModelReadPolicy.navigation(state)
                                        Box {
                                            OutlinedButton(onClick={modelMenu=true},enabled=enabled && catalog.entries.isNotEmpty()) {
                                                Text("Model catalog");Icon(Icons.Outlined.ArrowDropDown,contentDescription="Select host model from catalog")
                                            }
                                            DropdownMenu(expanded=modelMenu && enabled,onDismissRequest={modelMenu=false}) {
                                                catalog.entries.forEach { choice -> DropdownMenuItem(enabled=selectionEnabled && selected?.modelControl == true && choice.selectable,
                                                    text={Column(Modifier.widthIn(max=240.dp)){Text(choice.id,maxLines=2,overflow=TextOverflow.Ellipsis);Text(choice.status,style=MaterialTheme.typography.labelSmall)}},
                                                    onClick={modelInput=choice.id;modelMenu=false}) }
                                            }
                                        }
                                        Row {
                                            IconButton(onClick={onModelPage(state.modelPrevious.lastOrNull()?.catalog?.offset ?: (catalog.offset-16).coerceAtLeast(0))},enabled=enabled && state.modelPrevious.isNotEmpty()) {
                                                Icon(Icons.Outlined.ArrowBack,contentDescription="Previous model page")
                                            }
                                            IconButton(onClick={onModelPage(requireNotNull(catalog.nextOffset))},enabled=enabled && catalog.nextOffset!=null) {
                                                Icon(Icons.Outlined.ArrowForward,contentDescription="Next model page")
                                            }
                                        }
                                    }
                                HorizontalDivider()
                                if (status.capabilities == null) Text("Capabilities unavailable")
                                status.capabilities?.forEach { (name, available) ->
                                    Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                        Icon(if (available) Icons.Outlined.Check else Icons.Outlined.Close, contentDescription = null)
                                        Spacer(Modifier.width(8.dp)); Text(name.replace('_', ' '), modifier = Modifier.weight(1f))
                                    }
                                }
                            }
                        } }, confirmButton = { Row {
                            IconButton(onClick = onModel, enabled = ModelReadPolicy.browse(state)) {
                                Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host model")
                            }
                            IconButton(onClick = onCloseModel) { Icon(Icons.Outlined.Close, contentDescription = "Close host model") }
                        } })
                    if (projection.projects.any { it.id == projection.selectedId && it.promptControl }) {
                        val activePrompt = selected?.promptStatuses?.firstOrNull { it.state in setOf("accepted", "running") }
                        Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                            if (activePrompt == null) IconButton(onClick = { onResources(-1) },
                                enabled = ResourceReadPolicy.browse(state)) {
                                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Select host resource")
                            }
                            if (activePrompt == null && projection.subtabs != null) IconButton(onClick = { onFiles(".") },
                                enabled = FileReadPolicy.browse(state)) {
                                Icon(Icons.Outlined.AttachFile, contentDescription = "Attach host file")
                            }
                            OutlinedTextField(prompt, { if (it.toByteArray().size <= 65536) prompt = it }, label = { Text(if (activePrompt == null) "Message" else "Steer") },
                                modifier = Modifier.weight(1f).testTag("prompt-composer"), maxLines = 5,
                                enabled = if (activePrompt == null) PromptControlPolicy.send(state) else PromptControlPolicy.active(state, activePrompt.requestId))
                            IconButton(onClick = { keyboard?.hide(); focus.clearFocus(); if (activePrompt == null) onPrompt(prompt) else onSteer(activePrompt.requestId, prompt) },
                                enabled = prompt.isNotBlank() && !prompt.trimStart().startsWith("!")
                                    && (activePrompt == null || zenpi.transport.SteeringInput.valid(prompt))
                                    && !prompt.contains('\u0000')
                                    && (if (activePrompt == null) PromptControlPolicy.send(state) else PromptControlPolicy.active(state, activePrompt.requestId))) {
                                Icon(Icons.Outlined.Send, contentDescription = if (activePrompt == null) "Send message" else "Send steering input")
                            }
                        }
                    }
                    if (selected?.workerControl == true && currentSubtabs != null) {
                        val tab = currentSubtabs.tabs.single { it.id == currentSubtabs.activeId }
                        Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                            OutlinedTextField(workerPrompt, { if (it.toByteArray().size <= 65536) workerPrompt = it },
                                label = { Text("Task: ${tab.name}", maxLines = 1, overflow = TextOverflow.Ellipsis) },
                                modifier = Modifier.weight(1f), maxLines = 5, enabled = PromptControlPolicy.workerSend(state))
                            IconButton(onClick = { keyboard?.hide(); focus.clearFocus(); onWorkerPrompt(workerPrompt) },
                                enabled = workerPrompt.isNotBlank() && !workerPrompt.contains('\u0000') && !workerPrompt.trimStart().startsWith("!")
                                    && PromptControlPolicy.workerSend(state)) {
                                Icon(Icons.Outlined.Send, contentDescription = "Send worktree task")
                            }
                        }
                    }
                    if (state.resourcesOpen) AlertDialog(onDismissRequest = onCloseResources, title = { Text("Host resources") },
                        text = {
                            val scroll = rememberScrollState()
                            LaunchedEffect(state.resources?.snapshotId, state.resources?.generation, state.resources?.offset) { scroll.scrollTo(0) }
                            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll).testTag("resource-catalog-list")) {
                            val resourcesEnabled = ResourceReadPolicy.browse(state)
                            if (state.resourcesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                            state.resourcesError?.let { Text(it, color = MaterialTheme.colorScheme.error)
                                IconButton(onClick = { onResources(-1) }, enabled = resourcesEnabled) {
                                    Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host resources")
                                }
                            }
                            state.resources?.let { catalog ->
                                val selectionEnabled = ResourceReadPolicy.navigation(state)
                                if (selected?.resourcesReloadControl == true) {
                                    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                                        tooltip = { PlainTooltip { Text("Reload host resources") } }, state = rememberTooltipState()) {
                                        IconButton(onClick = onReloadResources, enabled = selectionEnabled) {
                                            Icon(Icons.Outlined.Refresh, contentDescription = "Reload host resources")
                                        }
                                    }
                                }
                                if (catalog.entries.isEmpty()) Text("No resources")
                                catalog.entries.forEach { resource ->
                                    TextButton(onClick = {
                                        runCatching { zenpi.transport.ResourceCatalogCodec.insert(resource, prompt) }.getOrNull()?.let {
                                            prompt = it; onCloseResources()
                                        }
                                    }, enabled = ResourceReadPolicy.insertion(state) && resource.invocable && !prompt.trimStart().startsWith('/')
                                        && !prompt.trimStart().startsWith('!') && (resource.command + " " + prompt).toByteArray().size <= 65536) {
                                        Column(Modifier.fillMaxWidth()) {
                                            Text(resource.command, maxLines = 2, overflow = TextOverflow.Ellipsis)
                                            if (resource.description.isNotEmpty()) Text(resource.description, style = MaterialTheme.typography.bodySmall,
                                                maxLines = 3, overflow = TextOverflow.Ellipsis)
                                            if (!resource.invocable) Text("Reload required", style = MaterialTheme.typography.labelSmall)
                                        }
                                    }
                                }
                            }
                        } }, confirmButton = {
                            state.resources?.let { catalog ->
                                val selectionEnabled = ResourceReadPolicy.navigation(state)
                                Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                    IconButton(onClick = { onResources(state.resourcesPrevious.lastOrNull()?.offset ?: (catalog.offset - 32).coerceAtLeast(0)) }, enabled = selectionEnabled && state.resourcesPrevious.isNotEmpty()) {
                                        Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous resource page")
                                    }
                                    Text("${catalog.offset / 32 + 1}", modifier = Modifier.width(32.dp),
                                        textAlign = androidx.compose.ui.text.style.TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                                    IconButton(onClick = { catalog.nextOffset?.let(onResources) }, enabled = selectionEnabled && catalog.nextOffset != null) {
                                        Icon(Icons.Outlined.ArrowForward, contentDescription = "Next resource page")
                                    }
                                }
                            }
                        }, dismissButton = { IconButton(onClick = onCloseResources) { Icon(Icons.Outlined.Close, contentDescription = "Close host resources") } })
                    if (state.filesOpen) AlertDialog(onDismissRequest = onCloseFiles, title = { Text("Host files") },
                        text = {
                            val scroll = rememberScrollState()
                            LaunchedEffect(state.files?.path, state.files?.page?.snapshotId, state.files?.page?.offset) { scroll.scrollTo(0) }
                            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll).testTag("workspace-files-list")) {
                            val filesEnabled = FileReadPolicy.browse(state)
                            if (state.filesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                            state.filesError?.let { Text(it, color = MaterialTheme.colorScheme.error)
                                IconButton(onClick = { onFiles(".") }, enabled = filesEnabled) {
                                    Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host files")
                                }
                            }
                            state.files?.let { files ->
                                val selectionEnabled = FileReadPolicy.navigation(state)
                                Text(files.path, style = MaterialTheme.typography.labelMedium)
                                if (files.path != ".") IconButton(onClick = { onFiles(files.path.substringBeforeLast('/', ".")) }, enabled = selectionEnabled) {
                                    Icon(Icons.Outlined.ArrowBack, contentDescription = "Parent directory")
                                }
                                if (files.entries.isEmpty()) Text("No files")
                                files.entries.forEach { file ->
                                    TextButton(onClick = {
                                        if (file.directory) onFiles(file.path) else {
                                            val reference = zenpi.transport.WorkspaceFilesCodec.reference(file)
                                            val next = prompt + (if (prompt.isEmpty() || prompt.last().isWhitespace()) "" else " ") + reference
                                            if (next.toByteArray().size <= 65536) { prompt = next; onCloseFiles() }
                                        }
                                    }, enabled = selectionEnabled && (file.directory || file.attachable && FileReadPolicy.attachment(state))) {
                                        Icon(if (file.directory) Icons.Outlined.Folder else Icons.Outlined.InsertDriveFile, contentDescription = null)
                                        Spacer(Modifier.width(8.dp)); Text(file.name, maxLines = 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                                    }
                                }
                                if (files.truncated) Text("File list truncated", style = MaterialTheme.typography.labelSmall)
                            }
                        } }, confirmButton = {
                            state.files?.page?.let { page ->
                                val selectionEnabled = FileReadPolicy.navigation(state)
                                Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                        IconButton(onClick={onFilePage(state.filesPrevious.lastOrNull()?.page?.offset ?: (page.offset-64).coerceAtLeast(0))},enabled=selectionEnabled && state.filesPrevious.isNotEmpty()) {
                                            Icon(Icons.Outlined.ArrowBack,contentDescription="Previous files page")
                                        }
                                        Text("${page.offset/64+1}", modifier = Modifier.width(32.dp),
                                            textAlign = androidx.compose.ui.text.style.TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                                        IconButton(onClick={page.nextOffset?.let(onFilePage)},enabled=selectionEnabled && page.nextOffset!=null) {
                                            Icon(Icons.Outlined.ArrowForward,contentDescription="Next files page")
                                        }
                                }
                            }
                        }, dismissButton = { IconButton(onClick = onCloseFiles) { Icon(Icons.Outlined.Close, contentDescription = "Close host files") } })
                    projection.turns.forEach { turn ->
                        Text(turn.role, style = MaterialTheme.typography.labelSmall)
                        Text(turn.content, style = MaterialTheme.typography.bodyMedium)
                        turn.toolCalls.forEach { call ->
                            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                Text(call.name, style = MaterialTheme.typography.labelMedium,
                                    modifier = Modifier.weight(1f).testTag("transcript-tool-${call.id}"))
                                TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                                    tooltip = { PlainTooltip { Text("Read tool arguments") } }, state = rememberTooltipState()) {
                                    IconButton(onClick = { onToolArguments(turn.id, call.id) }, enabled = TranscriptReadPolicy.tool(state, turn.id, call.id)) {
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
                            IconButton(onClick = { onTranscript(turn.id, false) }, enabled = TranscriptReadPolicy.browse(state)) {
                                Icon(Icons.Outlined.ArrowForward, contentDescription = "Read full message ${turn.id}")
                            }
                        }
                        HorizontalDivider()
                    }
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
                    if (state.phase == PairingPhase.CONNECTED && (selected?.terminal == null || selected?.terminal?.running == false) && selected?.terminalOpenControl == true) {
                        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                            tooltip = { PlainTooltip { Text("Open desktop terminal") } }, state = rememberTooltipState()) {
                            IconButton(onClick = onOpenDesktopTerminal, enabled = !state.scanInProgress && !state.resetting && !state.resetRequired
                                && state.pendingWorkspace == null && state.pendingRevoke == null
                                && runCatching { zenpi.transport.TerminalCommandCodec.open(projection) }.isSuccess) {
                                Icon(Icons.Outlined.Terminal, contentDescription = "Open desktop terminal")
                            }
                        }
                    }
                    if (state.phase == PairingPhase.CONNECTED) selected?.terminal?.takeIf {
                        zenpi.transport.TerminalProjectionCodec.valid(it)
                            && zenpi.transport.SessionCatalogCodec.readable(projection)
                            && currentSubtabs != null
                            && it.projectId == selected.id && it.sessionId == selected.sessionId
                            && currentSubtabs.projectId == selected.id
                            && it.subtabId == currentSubtabs.activeId && it.subtabRevision == currentSubtabs.revision
                    }?.let { terminal ->
                        val terminalScroll = remember(hostScope, sessionScope, subtabScope, terminal.terminalId) { androidx.compose.foundation.ScrollState(0) }
                        HorizontalDivider()
                        Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                            Text("Desktop terminal", modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
                            if (terminal.terminalId != null) TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                                tooltip = { PlainTooltip { Text("Terminal history") } }, state = rememberTooltipState()) {
                                IconButton(onClick = { onTerminalPage(false) }, enabled = TerminalReadPolicy.browse(state)) {
                                    Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Read terminal history")
                                }
                            }
                        }
                        Box(Modifier.fillMaxWidth().horizontalScroll(terminalScroll).testTag("terminal-viewport")) {
                            androidx.compose.foundation.text.selection.SelectionContainer {
                                Text(terminal.text, modifier = Modifier.testTag("desktop-terminal"), softWrap = false,
                                    fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
                            }
                        }
                        if (terminal.truncated) Text("Terminal output truncated", style = MaterialTheme.typography.labelSmall)
                        if (!terminal.running) Text("Terminal exited", style = MaterialTheme.typography.labelSmall)
                        if (selected?.terminalControl == true && terminal.terminalId != null && terminal.running) {
                            val enabled = state.pendingWorkspace == null && state.pendingRevoke == null && !state.scanInProgress && !state.resetting && !state.resetRequired
                            var input by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId) { mutableStateOf("") }
                            var enter by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId) { mutableStateOf(true) }
                            var rows by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId, terminal.size) { mutableStateOf((terminal.size?.rows ?: 24).toString()) }
                            var cols by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId, terminal.size) { mutableStateOf((terminal.size?.cols ?: 80).toString()) }
                            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                OutlinedTextField(value = input, onValueChange = { if (it.toByteArray().size <= 4095 && !it.contains('\u0000')) input = it },
                                    modifier = Modifier.weight(1f).testTag("terminal-input"), label = { Text("Terminal input") }, enabled = enabled)
                                TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Send to terminal") } }, state = rememberTooltipState()) {
                                    IconButton(onClick = { onTerminalInput(input + if (enter) "\r" else ""); input = "" }, enabled = enabled && input.isNotEmpty()) {
                                        Icon(Icons.Outlined.Send, contentDescription = "Send terminal input")
                                    }
                                }
                            }
                            Row(verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                Checkbox(enter, { enter = it }, enabled = enabled)
                                Text("Enter", style = MaterialTheme.typography.labelMedium)
                            }
                            FlowRow {
                                var keysOpen by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId) { mutableStateOf(false) }
                                LaunchedEffect(enabled) { if (!enabled) keysOpen = false }
                                listOf(Triple("Enter", "\r", Icons.Outlined.KeyboardReturn), Triple("Tab", "\t", Icons.Outlined.KeyboardTab),
                                    Triple("Interrupt terminal", "\u0003", Icons.Outlined.Stop), Triple("Up", "\u001b[A", Icons.Outlined.ArrowUpward),
                                    Triple("Down", "\u001b[B", Icons.Outlined.ArrowDownward), Triple("Left", "\u001b[D", Icons.Outlined.ArrowBack),
                                    Triple("Right", "\u001b[C", Icons.Outlined.ArrowForward)).forEach { (name, bytes, icon) ->
                                    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text(name) } }, state = rememberTooltipState()) {
                                        IconButton(onClick = { onTerminalInput(bytes) }, enabled = enabled) { Icon(icon, contentDescription = name) }
                                    }
                                }
                                TextButton(onClick = { onTerminalInput("\u001b") }, enabled = enabled) { Text("Esc") }
                                Box {
                                    TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Terminal keys") } }, state = rememberTooltipState()) {
                                        IconButton(onClick = { keysOpen = true }, enabled = enabled) { Icon(Icons.Outlined.Keyboard, contentDescription = "Terminal keys") }
                                    }
                                    DropdownMenu(expanded = keysOpen && enabled, onDismissRequest = { keysOpen = false }) {
                                        zenpi.transport.TerminalKey.entries.forEach { key ->
                                            DropdownMenuItem(text = { Text(key.label) }, enabled = enabled, onClick = {
                                                keysOpen = false; onTerminalInput(key.bytes)
                                            })
                                        }
                                    }
                                }
                            }
                            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                OutlinedTextField(rows, { if (it.length <= 3 && it.all(Char::isDigit)) rows = it }, Modifier.weight(1f), label = { Text("Rows") }, singleLine = true, enabled = enabled)
                                OutlinedTextField(cols, { if (it.length <= 3 && it.all(Char::isDigit)) cols = it }, Modifier.weight(1f), label = { Text("Columns") }, singleLine = true, enabled = enabled)
                                TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Resize terminal") } }, state = rememberTooltipState()) {
                                    IconButton(onClick = { onTerminalResize(rows.toInt(), cols.toInt()) }, enabled = enabled && rows.toIntOrNull() in 1..512 && cols.toIntOrNull() in 1..512) {
                                        Icon(Icons.Outlined.OpenInFull, contentDescription = "Resize terminal")
                                    }
                                }
                            }
                        }
                    }
                    if (projection.truncated) Text("Transcript truncated", style = MaterialTheme.typography.labelSmall)
                }
            }
            state.invite?.let { invite ->
                HorizontalDivider()
                Text(invite.address, style = MaterialTheme.typography.titleMedium)
                Text("Host SHA-256", style = MaterialTheme.typography.labelMedium)
                Text(invite.fingerprint.chunked(16).joinToString("\n"), style = MaterialTheme.typography.bodySmall)
                if (state.phase == PairingPhase.CONFIRMING) {
                    Text("Expires in ${remaining}s", style = MaterialTheme.typography.bodySmall)
                    OutlinedTextField(value = code, onValueChange = { code = it.filter { digit -> digit in '0'..'9' }.take(6) },
                        label = { Text("Desktop safety code") }, singleLine = true,
                        enabled = state.canScan && remaining > 0,
                        keyboardOptions = KeyboardOptions(keyboardType = KeyboardType.Number, imeAction = ImeAction.Done),
                        keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() }), modifier = Modifier.fillMaxWidth())
                    Button(onClick = { onConfirm(code) }, enabled = PairingConfirmationPolicy.allows(state, code, now)) { Text("Confirm host") }
                }
            }
            if (state.phase == PairingPhase.ACCEPTED && state.registered && state.host != null) {
                Text("Device registered", style = MaterialTheme.typography.titleSmall)
                Text("Host control unavailable", color = MaterialTheme.colorScheme.secondary)
            }
            if (state.registered) {
                if (state.pendingRevoke != null) TextButton(onClick = onCheckRevoke, enabled = DeviceActionPolicy.checkRevocation(state)) { Text("Check revocation status") }
                TextButton(onClick = { revokeDialog = true }, enabled = DeviceActionPolicy.revoke(state)) { Text("Revoke device on host") }
                TextButton(onClick = { forgetDialog = true }, enabled = DeviceActionPolicy.forget(state)) { Text("Forget device locally") }
            }
            if (state.resetRequired && !state.resetting) TextButton(onClick = { forgetDialog = true }, enabled = DeviceActionPolicy.forget(state)) { Text("Reset local device") }
            state.error?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            if (state.workspaceFailureCode == "unknown_outcome") {
                Text("Desktop could not confirm the write. Retry the same request only after checking its result.",
                    color = MaterialTheme.colorScheme.error,
                    modifier = Modifier.testTag("workspace-unknown-outcome"))
            }
            if (state.pendingWorkspace != null && state.phase != PairingPhase.CONNECTING) {
                Button(onClick = onRetryWorkspace) { Icon(Icons.Outlined.Refresh, null); Spacer(Modifier.width(8.dp)); Text("Retry same request") }
            }
            if (state.phase == PairingPhase.CONNECTING) LinearProgressIndicator(Modifier.fillMaxWidth())
            if (state.phase != PairingPhase.OFFLINE && !state.resetting && !state.resetRequired) TextButton(onClick = onCancel) { Text("Cancel") }
    }
    if (revokeDialog) AlertDialog(onDismissRequest = { revokeDialog = false }, title = { Text("Revoke this device?") },
        text = { Text("This device will lose access to the host. Local keys are removed only after host confirmation.") },
        confirmButton = { TextButton(enabled = DeviceActionPolicy.revoke(state),
            onClick = { revokeDialog = false; onRevoke() }) { Text("Revoke") } },
        dismissButton = { TextButton(onClick = { revokeDialog = false }) { Text("Cancel") } })
}
    if (forgetDialog) AlertDialog(onDismissRequest = { forgetDialog = false }, title = { Text("Remove local device data?") },
        text = { Text("Local keys, cached messages and unresolved request records will be removed. This does not revoke access on the desktop.") },
        confirmButton = { TextButton(enabled = DeviceActionPolicy.forget(state),
            onClick = { forgetDialog = false; onForget() }) { Text("Remove local data") } },
        dismissButton = { TextButton(onClick = { forgetDialog = false }) { Text("Cancel") } })
    if (state.diffOpen) {
        var path by rememberSaveable(hostScope, sessionScope, state.projection?.subtabs?.activeId) { mutableStateOf(".") }
        val diffEnabled = DiffReadPolicy.browse(state)
        AlertDialog(onDismissRequest = onCloseDiff, title = { Text("Diff") },
            text = {
                val dialogFocus = LocalFocusManager.current
                val dialogKeyboard = LocalSoftwareKeyboardController.current
                val scroll = rememberScrollState()
                LaunchedEffect(state.diff?.page?.offset) { scroll.scrollTo(0) }
                Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll),
                verticalArrangement = Arrangement.spacedBy(8.dp)) {
                OutlinedTextField(path, { if (it.toByteArray().size <= 4096) path = it }, label = { Text("Path") },
                    modifier = Modifier.fillMaxWidth(), singleLine = true,
                    keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                    enabled = diffEnabled,
                    keyboardActions = KeyboardActions(onDone = { dialogFocus.clearFocus(); dialogKeyboard?.hide(); if (diffEnabled) onDiff(path) }))
                if (state.diffLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.diffError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                state.diff?.let { diff ->
                    Text(diff.path, style = MaterialTheme.typography.labelLarge)
                    Text(when { diff.binary -> "Binary file"; !diff.changed -> "No changes"; diff.truncated -> "Diff truncated"; else -> "Changes" },
                        style = MaterialTheme.typography.labelMedium)
                    diff.page?.let { Text("${it.offset} - ${it.nextOffset} / ${it.totalBytes} bytes", style = MaterialTheme.typography.labelSmall) }
                    androidx.compose.foundation.text.selection.SelectionContainer {
                        Column { if (diff.status.isNotEmpty()) Text(diff.status, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
                            Text(diff.diff, modifier = Modifier.testTag("diff-patch"), fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall) }
                    }
                }
            } },
            confirmButton = { Row {
                val dialogFocus = LocalFocusManager.current
                val dialogKeyboard = LocalSoftwareKeyboardController.current
                val navigation = DiffReadPolicy.navigation(state)
                val previous = state.diff?.let { runCatching { zenpi.transport.DiffProjectionCodec.previous(state.diffPrevious, it) }.getOrNull() }
                IconButton(onClick = onPreviousDiff, enabled = navigation && previous != null) { Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous diff page") }
                IconButton(onClick = onNextDiff, enabled = navigation && state.diff?.page?.hasMore == true) { Icon(Icons.Outlined.ArrowForward, contentDescription = "Next diff page") }
                IconButton(onClick = { dialogFocus.clearFocus(); dialogKeyboard?.hide(); onDiff(path) }, enabled = diffEnabled && path.isNotBlank()) {
                Icon(Icons.Outlined.Refresh, contentDescription = "Refresh diff") } } },
            dismissButton = { IconButton(onClick = onCloseDiff) { Icon(Icons.Outlined.Close, contentDescription = "Close diff") } })
    }
    if (state.workerStatusesOpen) AlertDialog(onDismissRequest = onCloseWorkerStatuses, title = { Text("Worktree tasks") }, text = {
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (state.workerStatusesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.workerStatusesError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.workerStatusesPage?.let { page ->
                Text("${page.offset} - ${page.nextOffset} / ${page.totalTasks}", style = MaterialTheme.typography.labelSmall)
                page.tasks.forEach { task ->
                    Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                        Column(Modifier.weight(1f)) {
                            Text(task.requestId, style = MaterialTheme.typography.labelMedium, maxLines = 3, overflow = TextOverflow.Ellipsis)
                            Text(task.state, style = MaterialTheme.typography.bodySmall)
                            if (task.runtimeActive) Text("Task runtime active", style = MaterialTheme.typography.labelSmall)
                        }
                        if (task.canCancel) IconButton(onClick = { onCancelWorker(task.requestId) },
                            enabled = !state.workerStatusesLoading && state.workerStatusesError == null
                                && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null
                                && WorkerTargetPolicy.cancellable(state, task.requestId) != null) {
                            Icon(Icons.Outlined.Stop, contentDescription = "Cancel listed task ${task.requestId}")
                        }
                        if (task.state == "succeeded") {
                            val readable = WorkerStatusReadPolicy.navigation(state) && WorkerReadPolicy.projection(state, task.requestId) != null
                            IconButton(onClick = { onWorkerTranscript(task.requestId, false) }, enabled = readable && TranscriptReadPolicy.browse(state)) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read listed task output ${task.requestId}")
                            }
                            IconButton(onClick = { onWorkerHistory(task.requestId, false) }, enabled = readable && !state.workerHistoryLoading) {
                                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Read listed task history ${task.requestId}")
                            }
                        }
                    }
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = WorkerStatusReadPolicy.navigation(state)
        IconButton(onClick = { onWorkerStatuses(-1) }, enabled = WorkerStatusReadPolicy.browse(state)) {
            Icon(Icons.Outlined.Refresh, contentDescription = "Refresh task list")
        }
        IconButton(onClick = { state.workerStatusesPage?.let { page ->
            onWorkerStatuses(state.workerStatusesPrevious.lastOrNull()?.offset ?: (page.offset - 16).coerceAtLeast(0))
        } }, enabled = enabled && state.workerStatusesPrevious.isNotEmpty()) {
            Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous task list page")
        }
        IconButton(onClick = { state.workerStatusesPage?.let { onWorkerStatuses(it.nextOffset) } }, enabled = enabled && state.workerStatusesPage?.hasMore == true) {
            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next task list page")
        }
    } }, dismissButton = { IconButton(onClick = onCloseWorkerStatuses) { Icon(Icons.Outlined.Close, contentDescription = "Close task list") } })
    if (state.sessionHistoryOpen && state.transcriptTurn == null) AlertDialog(onDismissRequest = onCloseSessionHistory,
        title = { Text("Conversation") }, text = {
            val scroll = rememberScrollState()
            LaunchedEffect(state.sessionHistory?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.sessionHistoryLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.sessionHistoryError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                val enabled = zenpi.pairing.SessionHistoryReadPolicy.navigation(state)
                state.sessionHistory?.let { page ->
                    Text("${page.offset}-${page.nextOffset} / ${page.totalTurns}", style = MaterialTheme.typography.labelSmall)
                    page.turns.forEach { turn ->
                        Text(turn.role, style = MaterialTheme.typography.labelSmall)
                        Text(turn.content, style = MaterialTheme.typography.bodySmall)
                        IconButton(onClick = { onSessionHistoryTurn(turn.id) }, enabled = enabled) {
                            Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation message ${turn.id}")
                        }
                        turn.toolCalls.forEach { call ->
                            Text(call.name, style = MaterialTheme.typography.labelSmall)
                            Text(call.argumentsPreview, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                            IconButton(onClick = { onSessionHistoryTool(turn.id, call.id) }, enabled = enabled) {
                                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read conversation tool arguments ${call.id}")
                            }
                        }
                        HorizontalDivider()
                    }
                }
            }
        }, confirmButton = {
            val enabled = zenpi.pairing.SessionHistoryReadPolicy.navigation(state)
            Row {
                IconButton(onClick = { onSessionHistory(false) }, enabled = SessionHistoryReadPolicy.browse(state)) {
                    Icon(Icons.Outlined.Refresh, contentDescription = "Refresh conversation")
                }
                IconButton(onClick = onPreviousSessionHistory, enabled = enabled && state.sessionHistoryPrevious.isNotEmpty()) {
                    Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous conversation page")
                }
                IconButton(onClick = { onSessionHistory(true) }, enabled = enabled && state.sessionHistory?.hasMore == true) {
                    Icon(Icons.Outlined.ArrowForward, contentDescription = "Next conversation page")
                }
            }
        }, dismissButton = { IconButton(onClick = onCloseSessionHistory) { Icon(Icons.Outlined.Close, contentDescription = "Close conversation") } })
    state.workerHistoryRequest?.takeIf { state.transcriptTurn == null }?.let { requestId ->
        val historyEnabled = WorkerHistoryNavigation.navigation(state)
        AlertDialog(onDismissRequest = onCloseWorkerHistory, title = { Text("Task history") }, text = {
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(workerHistoryScroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.workerHistoryLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.workerHistoryError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                state.workerHistoryPage?.let { page ->
                    Text("${page.offset} - ${page.nextOffset} / ${page.totalTurns}", style = MaterialTheme.typography.labelSmall)
                    page.turns.forEach { turn ->
                        Text(turn.role, style = MaterialTheme.typography.labelSmall)
                        androidx.compose.foundation.text.selection.SelectionContainer { Text(turn.content, style = MaterialTheme.typography.bodyMedium) }
                        turn.toolCalls.forEach { call ->
                            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                                Text(call.name, modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
                                IconButton(onClick = { onWorkerToolArguments(turn.id, call.id) }, enabled = historyEnabled) {
                                    Icon(Icons.Outlined.OpenInFull, contentDescription = "Read tool arguments ${call.id}")
                                }
                            }
                            androidx.compose.foundation.text.selection.SelectionContainer {
                                Text(call.argumentsPreview + if (call.argumentsTruncated) "..." else "", style = MaterialTheme.typography.bodySmall,
                                    fontFamily = androidx.compose.ui.text.font.FontFamily.Monospace)
                            }
                        }
                        if (turn.toolCallsTruncated) Text("...", style = MaterialTheme.typography.labelSmall)
                        IconButton(onClick = { onWorkerHistoryTurn(turn.id) }, enabled = historyEnabled) {
                            Icon(Icons.Outlined.OpenInFull, contentDescription = "Read task message ${turn.id}")
                        }
                    }
                }
            }
        }, confirmButton = { Row {
            IconButton(onClick = onPreviousWorkerHistory, enabled = historyEnabled && state.workerHistoryPrevious.isNotEmpty()) {
                Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous task history page")
            }
            IconButton(onClick = { onWorkerHistory(requestId, true) }, enabled = historyEnabled && state.workerHistoryPage?.hasMore == true) {
            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next task history page")
        } } }, dismissButton = { IconButton(onClick = onCloseWorkerHistory) { Icon(Icons.Outlined.Close, contentDescription = "Close task history") } })
    }
    newSessionTarget?.let { target ->
        val selected = state.projection?.projects?.singleOrNull { it.id == target.first && it.sessionId == target.second && it.sessionControl }
        val enabled = selected != null && state.projection?.selectedId == target.first && state.projection?.sessionId == target.second
            && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(state.projection)
            && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null
            && state.projection?.phase == "idle" && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
        AlertDialog(onDismissRequest = { newSessionTarget = null }, title = { Text("Start a new session?") },
            text = { Text("The current conversation remains on the desktop. A new empty conversation will become active.") },
            confirmButton = { IconButton(onClick = { newSessionTarget = null; onNewSession() }, enabled = enabled) { Icon(Icons.Outlined.Check, contentDescription = "Confirm new session") } },
            dismissButton = { IconButton(onClick = { newSessionTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel new session") } })
    }
    renameSessionTarget?.let { target ->
        val project = state.projection?.projects?.singleOrNull { it.id == target.first && it.sessionId == target.second && it.sessionControl }
        val enabled = project != null && state.projection?.selectedId == target.first && state.projection?.sessionId == target.second
            && !state.scanInProgress && !state.resetting && !state.resetRequired && zenpi.transport.SessionCatalogCodec.readable(state.projection)
            && state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null && state.pendingRevoke == null && state.projection?.phase == "idle"
            && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
        val valid = runCatching { zenpi.transport.SessionCommandCodec.validateName(sessionNameDraft) }.isSuccess
        AlertDialog(onDismissRequest = { renameSessionTarget = null }, title = { Text("Rename session") }, text = {
            OutlinedTextField(value = sessionNameDraft, onValueChange = { sessionNameDraft = it }, label = { Text("Session name") },
                singleLine = true, isError = sessionNameDraft.isNotEmpty() && !valid, modifier = Modifier.fillMaxWidth())
        }, confirmButton = { IconButton(onClick = { renameSessionTarget = null; onRenameSession(sessionNameDraft) }, enabled = enabled && valid) {
            Icon(Icons.Outlined.Check, contentDescription = "Confirm session rename")
        } }, dismissButton = { IconButton(onClick = { renameSessionTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel session rename") } })
    }
    if (state.sessionsOpen) AlertDialog(onDismissRequest = onCloseSessions, title = { Text("Sessions") }, text = {
        val scroll = rememberScrollState()
        LaunchedEffect(state.sessions?.offset) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            val searchEnabled = SessionCatalogReadPolicy.browse(state)
            Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
                OutlinedTextField(value = sessionSearch, onValueChange = { sessionSearch = it }, label = { Text("Search sessions") },
                    singleLine = true, enabled = searchEnabled, modifier = Modifier.weight(1f).testTag("session-search"))
                IconButton(onClick = { resumeTarget = null; onSearchSessions(sessionSearch) }, enabled = searchEnabled
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
                Row(Modifier.fillMaxWidth(), verticalAlignment = androidx.compose.ui.Alignment.CenterVertically) {
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
                    IconButton(onClick = { resumeTarget = null; onResumeSession(target) }, enabled = enabled
                        && state.sessions?.let { zenpi.transport.SessionCatalogCodec.resumable(it, state.projection, target) } == true) { Icon(Icons.Outlined.Check, contentDescription = "Confirm resume session") }
                    IconButton(onClick = { resumeTarget = null }) { Icon(Icons.Outlined.Close, contentDescription = "Cancel resume session") }
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = SessionCatalogReadPolicy.browse(state)
        val navigation = SessionCatalogReadPolicy.navigation(state)
        IconButton(onClick = { state.sessionsPrevious.lastOrNull()?.offset?.let(onSessions) }, enabled = navigation && state.sessionsPrevious.isNotEmpty()) {
            Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous sessions page")
        }
        IconButton(onClick = { state.sessions?.nextOffset?.let(onSessions) }, enabled = navigation && state.sessions?.nextOffset != null) {
            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next sessions page")
        }
        IconButton(onClick = { onSessions(-1) }, enabled = enabled) { Icon(Icons.Outlined.Refresh, contentDescription = "Refresh sessions") }
    } }, dismissButton = { IconButton(onClick = onCloseSessions) { Icon(Icons.Outlined.Close, contentDescription = "Close sessions") } })
    if (state.terminalOpen) AlertDialog(onDismissRequest = onCloseTerminal, title = { Text("Terminal history") }, text = {
        val scroll = rememberScrollState()
        LaunchedEffect(state.terminalPage?.offset) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
            if (state.terminalLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.terminalError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.terminalPage?.let { page ->
                Text("${page.offset} - ${page.nextOffset} / ${page.totalBytes} bytes", style = MaterialTheme.typography.labelSmall)
                androidx.compose.foundation.text.selection.SelectionContainer {
                    Text(page.text, modifier = Modifier.fillMaxWidth().testTag("terminal-history-page"), fontFamily = FontFamily.Monospace,
                        style = MaterialTheme.typography.bodySmall)
                }
            }
        }
    }, confirmButton = { Row {
        val enabled = TerminalReadPolicy.browse(state)
        val navigationEnabled = TerminalReadPolicy.navigation(state)
        val previous = state.terminalPage?.let { runCatching { zenpi.transport.TerminalPageCodec.previous(state.terminalPrevious, it) }.getOrNull() }
        IconButton(onClick = onPreviousTerminalPage, enabled = navigationEnabled && previous != null) {
            Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous terminal history page")
        }
        IconButton(onClick = { onTerminalPage(true) }, enabled = navigationEnabled && state.terminalPage?.hasMore == true) {
            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next terminal history page")
        }
        IconButton(onClick = { onTerminalPage(false) }, enabled = enabled && zenpi.transport.TerminalPageCodec.terminal(state.projection) != null) { Icon(Icons.Outlined.Refresh, contentDescription = "Refresh terminal history") }
    } }, dismissButton = { IconButton(onClick = onCloseTerminal) { Icon(Icons.Outlined.Close, contentDescription = "Close terminal history") } })
    state.transcriptTurn?.let { turnId ->
        AlertDialog(onDismissRequest = onCloseTranscript, title = { Text(state.transcriptToolCall?.name ?: "Message") },
            text = { val pageScroll = rememberScrollState()
                LaunchedEffect(state.transcriptPage?.offset) { pageScroll.scrollTo(0) }
                Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(pageScroll), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                if (state.transcriptLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.transcriptError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                state.transcriptPage?.let { page ->
                    Text("${page.offset} - ${page.nextOffset} / ${page.totalBytes} bytes", style = MaterialTheme.typography.labelSmall)
                    androidx.compose.foundation.text.selection.SelectionContainer {
                        Text(page.text, modifier = Modifier.testTag("transcript-page"), style = MaterialTheme.typography.bodyMedium,
                            fontFamily = if (state.transcriptToolCall != null) FontFamily.Monospace else FontFamily.Default)
                    }
                }
            } }, confirmButton = { Row {
                val navigation = TranscriptReadPolicy.navigation(state)
                IconButton(onClick = onPreviousTranscript, enabled = TranscriptReadPolicy.previous(state) != null) {
                    Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous message page")
                }
                IconButton(onClick = { if (state.transcriptWorker) onWorkerTranscript(turnId, true) else onTranscript(turnId, true) }, enabled = navigation && state.transcriptPage?.hasMore == true) {
                Icon(Icons.Outlined.ArrowForward, contentDescription = "Next message page")
            } } }, dismissButton = { IconButton(onClick = onCloseTranscript) { Icon(Icons.Outlined.Close, contentDescription = "Close message") } })
    }
    approvalDecision?.let { (target, decision) ->
        val enabled = ApprovalDecisionPolicy.allows(state, target)
        AlertDialog(onDismissRequest = { approvalDecision = null }, title = { Text(if (decision == "allow") "Allow this tool once?" else "Deny this tool?") },
            text = { Column(Modifier.heightIn(max = 360.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(8.dp)) {
                Text(target.tool)
                target.preview?.let { preview ->
                    Text(preview.path, style = MaterialTheme.typography.labelLarge)
                    if (preview.withheld) Text("Preview withheld", color = MaterialTheme.colorScheme.error)
                    else Text(preview.patch, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace)
                    if (preview.redacted && !preview.withheld) Text("Sensitive values redacted", style = MaterialTheme.typography.labelSmall)
                    if (preview.truncated) Text("Preview truncated", color = MaterialTheme.colorScheme.error)
                }
                Text(target.requestId, style = MaterialTheme.typography.labelSmall)
                target.workerSessionId?.let { Text("Worker $it", style = MaterialTheme.typography.labelSmall) }
                Text(target.digest.chunked(16).joinToString("\n"), style = MaterialTheme.typography.labelSmall)
            } },
            confirmButton = { TextButton(enabled = enabled, onClick = { onApprove(target, decision); approvalDecision = null }) { Text(if (decision == "allow") "Allow once" else "Deny") } },
            dismissButton = { TextButton(onClick = { approvalDecision = null }) { Text("Cancel") } })
    }
    if (openDialog) AlertDialog(onDismissRequest = { openDialog = false }, title = { Text("Open workspace") },
        text = { OutlinedTextField(directory, { directory = it }, label = { Text("Host directory") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() })) },
        confirmButton = { TextButton(enabled = workspaceEnabled && directory.isNotBlank(), onClick = {
            state.projection?.selectedId?.let { onWorkspace("workspace_open", it, directory) }; openDialog = false
        }) { Text("Open") } }, dismissButton = { TextButton(onClick = { openDialog = false }) { Text("Cancel") } })
    closeProject?.let { id -> AlertDialog(onDismissRequest = { closeProject = null }, title = { Text("Close workspace?") },
        confirmButton = { TextButton(enabled = workspaceEnabled && state.projection?.selectedId == id && (state.projection?.projects?.size ?: 0) > 1,
            onClick = { onWorkspace("workspace_close", id, null); closeProject = null }) { Text("Close") } },
        dismissButton = { TextButton(onClick = { closeProject = null }) { Text("Cancel") } }) }
    renameSubtab?.let { id -> AlertDialog(onDismissRequest = { renameSubtab = null }, title = { Text("Rename worktree tab") },
        text = { OutlinedTextField(subtabName, { subtabName = it }, label = { Text("Tab name") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() })) },
        confirmButton = { TextButton(enabled = subtabName.trim().isNotEmpty() && subtabName.toByteArray().size <= 128
            && subtabEnabled,
            onClick = { onSubtab("subtab_rename", id, subtabName.trim()); renameSubtab = null }) { Text("Rename") } },
        dismissButton = { TextButton(onClick = { renameSubtab = null }) { Text("Cancel") } }) }
    if (directoryTabDialog) AlertDialog(onDismissRequest = { directoryTabDialog = false }, title = { Text("Add directory tab") },
        text = { Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedTextField(directoryTabName, { directoryTabName = it }, label = { Text("Directory tab name") }, singleLine = true)
            OutlinedTextField(directoryTabPath, { directoryTabPath = it }, label = { Text("Project subdirectory") }, singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() }))
        } }, confirmButton = { TextButton(enabled = directoryTabName.trim().isNotEmpty() && directoryTabPath.isNotBlank()
            && subtabEnabled && state.projection?.subtabs?.directoryControl == true,
            onClick = { onAddDirectoryTab(directoryTabName.trim(), directoryTabPath); directoryTabDialog = false }) { Text("Add") } },
        dismissButton = { TextButton(onClick = { directoryTabDialog = false }) { Text("Cancel") } })
    if (worktreeDialog) AlertDialog(onDismissRequest = { worktreeDialog = false }, title = { Text("Add Git worktree") },
        text = { OutlinedTextField(worktreeName, { worktreeName = it }, label = { Text("New branch name") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { focus.clearFocus(); keyboard?.hide() })) },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.worktreeControl == true
            && worktreeName.matches(Regex("[A-Za-z0-9_][A-Za-z0-9_-]{0,127}")), onClick = {
            onAddWorktreeTab(worktreeName); worktreeDialog = false
        }) { Text("Create") } }, dismissButton = { TextButton(onClick = { worktreeDialog = false }) { Text("Cancel") } })
    closeWorktree?.let { id -> AlertDialog(onDismissRequest = { closeWorktree = null }, title = { Text("Remove Git worktree?") },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.worktreeControl == true,
            onClick = { onSubtab("subtab_close", id, null); closeWorktree = null }) { Text("Remove") } },
        dismissButton = { TextButton(onClick = { closeWorktree = null }) { Text("Cancel") } }) }
    closeDirectoryTab?.let { id -> AlertDialog(onDismissRequest = { closeDirectoryTab = null }, title = { Text("Close directory tab?") },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.directoryControl == true,
            onClick = { onSubtab("subtab_close", id, null); closeDirectoryTab = null }) { Text("Close") } },
        dismissButton = { TextButton(onClick = { closeDirectoryTab = null }) { Text("Cancel") } }) }
}
