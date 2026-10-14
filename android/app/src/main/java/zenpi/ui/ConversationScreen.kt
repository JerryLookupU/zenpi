package zenpi.ui

import zenpi.ui.theme.asHeading
import zenpi.ui.theme.spokenAs
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.foundation.layout.size
import androidx.compose.foundation.lazy.LazyColumn
import androidx.compose.foundation.lazy.LazyListState
import androidx.compose.foundation.lazy.itemsIndexed
import androidx.compose.foundation.lazy.rememberLazyListState
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.selection.selectableGroup
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.KeyboardArrowRight
import androidx.compose.material.icons.outlined.AccountTree
import androidx.compose.material.icons.outlined.ArrowDropDown
import androidx.compose.material.icons.outlined.KeyboardArrowDown
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material3.AssistChip
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.FilterChip
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.TopAppBar
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.snapshots.Snapshot
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.disabled
import androidx.compose.ui.semantics.onClick
import androidx.compose.ui.semantics.role
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.semantics.stateDescription
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.ModelReadPolicy
import zenpi.pairing.PairingPhase
import zenpi.pairing.PairingState
import zenpi.pairing.PromptControlPolicy
import zenpi.pairing.TranscriptReadPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemoteToolCall
import zenpi.transport.RemoteTurn
import zenpi.ui.nav.Route

/**
 * The main screen for an authorized host: a top bar (switcher, Agent/Arch mode, worktree and
 * model chips), the conversation as a reverse [LazyColumn] that sticks to the newest turn and
 * its streaming row and message statuses, and the bottom-pinned [ApprovalPanel] bar and [Composer]. The
 * chip row ends with the session actions that open the secondary pages ([onRoute]); [header] (the host
 * and device controls when the switcher is inline) scrolls in as the oldest list item.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ConversationScreen(scope: ScreenScope, projection: HostProjection, prompt: Draft, workspaceUi: WorkspaceUiState, actions: PairingActions,
    navigation: @Composable () -> Unit, onOpenSwitcher: () -> Unit, onRoute: (Route) -> Unit, status: @Composable () -> Unit,
    header: @Composable ColumnScope.() -> Unit) {
    val state = scope.state
    val selected = scope.selected
    val list = rememberLazyListState()
    val composer = projection.projects.any { it.id == projection.selectedId && it.promptControl }
    Scaffold(contentWindowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Horizontal),
        topBar = { ConversationTopBar(scope, projection, workspaceUi, actions, navigation, onOpenSwitcher, onRoute, status) },
        bottomBar = {
            // The keyboard lifts the composer, and shrinks the list above it, whether or not a composer is shown.
            Surface(tonalElevation = if (composer) 3.dp else 0.dp) {
                Column(Modifier.navigationBarsPadding().imePadding()) {
                    ApprovalPanel(scope, actions)
                    // Scrollable so a 5-line draft at a large font never pushes the composer off a small screen.
                    if (composer) Column(Modifier.heightIn(max = 280.dp).verticalScroll(rememberScrollState())
                        .padding(horizontal = 12.dp, vertical = 8.dp)) {
                        Composer(state, projection, prompt, actions)
                    }
                }
            }
        }) { padding ->
        ConversationList(state, projection, selected, list, padding, actions, header)
    }
    if (state.modelOpen) ModelSheet(state, projection, selected, actions)
    if (state.resourcesOpen) CommandPalette(scope, prompt, workspaceUi, actions)
    if (state.filesOpen) FilePalette(state, prompt, actions)
}

@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ConversationTopBar(scope: ScreenScope, projection: HostProjection, workspaceUi: WorkspaceUiState, actions: PairingActions,
    navigation: @Composable () -> Unit, onOpenSwitcher: () -> Unit, onRoute: (Route) -> Unit, status: @Composable () -> Unit) {
    Surface { Column(Modifier.padding(bottom = 4.dp)) {
        TopAppBar(title = { Column {
            Text(scope.selected?.title ?: "Zenpi", maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.asHeading())
            scope.selected?.sessionName?.let { Text(it, style = MaterialTheme.typography.labelMedium, maxLines = 1, overflow = TextOverflow.Ellipsis) }
        } },
            navigationIcon = navigation, windowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
        Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).padding(horizontal = 16.dp),
            horizontalArrangement = Arrangement.spacedBy(8.dp), verticalAlignment = Alignment.CenterVertically) {
            ModeSwitch()
            scope.currentSubtabs?.let { tabs -> tabs.tabs.singleOrNull { it.id == tabs.activeId } }?.let { tab ->
                // One spoken label, so the chip never doubles as a second "tab name" node beside the switcher's tabs.
                AssistChip(onClick = onOpenSwitcher, label = { Text(tab.name, maxLines = 1, overflow = TextOverflow.Ellipsis) },
                    leadingIcon = { Icon(Icons.Outlined.AccountTree, null, Modifier.size(18.dp)) },
                    modifier = Modifier.testTag("worktree-chip").clearAndSetSemantics {
                        contentDescription = "Worktree ${tab.name}, open switcher"; role = Role.Button
                        onClick { onOpenSwitcher(); true }
                    })
            }
            val model = projection.modelStatus?.model
            val modelEnabled = ModelReadPolicy.browse(scope.state)
            AssistChip(onClick = actions::model, enabled = modelEnabled,
                label = { Text(model ?: "Model", maxLines = 1, overflow = TextOverflow.Ellipsis) },
                trailingIcon = { Icon(Icons.Outlined.ArrowDropDown, null, Modifier.size(18.dp)) },
                modifier = Modifier.testTag("model-chip").clearAndSetSemantics {
                    contentDescription = "Inspect host model"; model?.let { stateDescription = it }; role = Role.Button
                    if (modelEnabled) onClick { actions.model(); true } else disabled()
                })
            SessionToolbar(scope, projection, workspaceUi, actions, onRoute)
        }
        Column(Modifier.padding(horizontal = 16.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) { status() }
    } }
}

/**
 * Agent is the only mode a host serves remotely; Arch (the desktop's master-session console) has
 * no remote operation yet, so it is shown, disabled, to keep the desktop's layout recognizable.
 */
@Composable
private fun ModeSwitch() {
    Row(Modifier.selectableGroup().testTag("mode-switch"), horizontalArrangement = Arrangement.spacedBy(4.dp)) {
        FilterChip(selected = true, onClick = {}, label = { Text("Agent") }, modifier = Modifier.semantics { role = Role.RadioButton })
        FilterChip(selected = false, onClick = {}, enabled = false, label = { Text("Arch") },
            modifier = Modifier.semantics { role = Role.RadioButton; stateDescription = "Desktop only" })
    }
}

@Composable
private fun ConversationList(state: PairingState, projection: HostProjection, selected: RemoteProject?, list: LazyListState,
    padding: PaddingValues, actions: ConversationActions, header: @Composable ColumnScope.() -> Unit) {
    val live = state.phase in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR)
        && (selected?.desktopStreamText != null || selected?.promptStatuses?.any { it.streamText != null } == true)
    // Follow the conversation only when the reader is already at the newest item: the check runs
    // against the layout from before this change, and without subscribing to every scroll.
    val newest = Triple(projection.turns.size, projection.turns.lastOrNull()?.id, live)
    val follow = remember(newest) { Snapshot.withoutReadObservation { list.firstVisibleItemIndex == 0 } }
    LaunchedEffect(newest) { if (follow) list.scrollToItem(0) }
    LazyColumn(Modifier.fillMaxSize().padding(padding).testTag("conversation"), state = list, reverseLayout = true,
        contentPadding = PaddingValues(horizontal = 16.dp, vertical = 12.dp), verticalArrangement = Arrangement.spacedBy(12.dp)) {
        if (selected?.promptStatuses?.isNotEmpty() == true) item(key = "statuses", contentType = "statuses") { PromptStatuses(state, projection, selected, actions) }
        if (live) item(key = "live", contentType = "live") { LiveStreams(selected) }
        val turns = projection.turns
        itemsIndexed(turns.asReversed(), key = { index, turn -> "turn-${turns.size - 1 - index}-${turn.id}" }, contentType = { _, _ -> "turn" }) { _, turn ->
            TurnRow(state, turn, actions)
        }
        item(key = "workspace", contentType = "workspace") {
            Column(verticalArrangement = Arrangement.spacedBy(16.dp)) {
                header()
                if (projection.truncated) Text("Transcript truncated", style = MaterialTheme.typography.labelSmall)
            }
        }
    }
}

@Composable
private fun TurnRow(state: PairingState, turn: RemoteTurn, actions: ConversationActions) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Text(turn.role, style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
        MarkdownText(turn.content)
        turn.toolCalls.forEach { call -> ToolRow(state, turn, call, actions) }
        if (turn.toolCallsTruncated) Text("Tool list truncated", style = MaterialTheme.typography.labelSmall)
        if (turn.contentTruncated) Row(verticalAlignment = Alignment.CenterVertically) {
            Text("Message truncated", style = MaterialTheme.typography.labelSmall, modifier = Modifier.weight(1f))
            IconButton(modifier = Modifier.testTag("Read full message ${turn.id}"), onClick = { actions.transcript(turn.id, false) }, enabled = TranscriptReadPolicy.browse(state)) {
                Icon(Icons.Outlined.OpenInFull, contentDescription = "Read full message")
            }
        }
        HorizontalDivider(Modifier.padding(top = 4.dp))
    }
}

/** A tool call collapsed to one line by default; the chevron reveals the argument preview. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
private fun ToolRow(state: PairingState, turn: RemoteTurn, call: RemoteToolCall, actions: ConversationActions) {
    var expanded by rememberSaveable(turn.id, call.id) { mutableStateOf(false) }
    Column {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            IconButton(onClick = { expanded = !expanded },
                modifier = Modifier.testTag((if (expanded) "Hide arguments " else "Show arguments ") + call.id)) {
                Icon(if (expanded) Icons.Outlined.KeyboardArrowDown else Icons.AutoMirrored.Outlined.KeyboardArrowRight,
                    contentDescription = (if (expanded) "Hide " else "Show ") + call.name + " arguments")
            }
            Text(call.name, style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis,
                modifier = Modifier.testTag("transcript-tool-${call.id}"))
            Text(if (expanded) "" else call.argumentsPreview.lineSequence().firstOrNull().orEmpty(), maxLines = 1, overflow = TextOverflow.Ellipsis,
                style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace,
                color = MaterialTheme.colorScheme.onSurfaceVariant, modifier = Modifier.weight(1f).padding(horizontal = 8.dp))
            TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                tooltip = { PlainTooltip { Text("Read tool arguments") } }, state = rememberTooltipState()) {
                IconButton(modifier = Modifier.testTag("Read tool arguments ${call.id}"), onClick = { actions.toolArguments(turn.id, call.id) }, enabled = TranscriptReadPolicy.tool(state, turn.id, call.id)) {
                    Icon(Icons.Outlined.OpenInFull, contentDescription = "Read tool arguments")
                }
            }
        }
        if (expanded) Text(call.argumentsPreview, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall,
            modifier = Modifier.fillMaxWidth().padding(start = 48.dp))
        if (call.argumentsTruncated) Text("Arguments truncated", style = MaterialTheme.typography.labelSmall, modifier = Modifier.padding(start = 48.dp))
    }
}

/** The streaming row: in-flight assistant text from phone-sent messages and from the desktop. */
@Composable
private fun LiveStreams(selected: RemoteProject?) {
    Column(verticalArrangement = Arrangement.spacedBy(4.dp)) {
        selected?.promptStatuses?.forEach { status ->
            status.streamText?.let { text ->
                Text("Assistant (in progress)", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
                MarkdownText(text, Modifier.testTag("live-prompt-${status.requestId}").semantics(mergeDescendants = true) {})
                if (status.streamTruncated) Text("Preview truncated", modifier = Modifier.testTag("live-prompt-truncated-${status.requestId}"),
                    style = MaterialTheme.typography.labelSmall)
            }
        }
        selected?.desktopStreamText?.let { text ->
            Text("Desktop assistant (in progress)", style = MaterialTheme.typography.labelSmall, color = MaterialTheme.colorScheme.primary)
            MarkdownText(text, Modifier.testTag("desktop-live-text").semantics(mergeDescendants = true) {})
        }
    }
}

private val activePromptStates = setOf("accepted", "running")

/** Status of the messages sent to the selected session, with a cancel for each running one. */
@Composable
private fun PromptStatuses(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: ConversationActions) {
    Column { projection.projects.find { it.id == projection.selectedId }?.promptStatuses?.forEach { status ->
        Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.SpaceBetween, verticalAlignment = Alignment.CenterVertically) {
            Text(status.requestId.take(8), style = MaterialTheme.typography.labelSmall,
                modifier = Modifier.testTag("prompt-request-${status.requestId}").spokenAs(null))
            Text(when (status.state) { "accepted" -> "Message accepted"; "running" -> "Message running"
                "succeeded" -> "Message succeeded"; "failed" -> when(status.failureCode){
                    "invalid_request"->"Message input rejected";"busy"->"Message owner busy";"resource_limit"->"Message limit reached"
                    "unauthorized","revoked_device"->"Message access denied";else->"Message failed"}; "cancelled" -> "Message cancelled"
                else -> "Message result unknown" }, style = MaterialTheme.typography.labelMedium,
                modifier=Modifier.weight(1f).padding(horizontal=8.dp),maxLines=2,overflow=TextOverflow.Ellipsis)
            if (status.state in activePromptStates && selected?.promptControl == true) {
                IconButton(modifier = Modifier.testTag("Cancel message ${status.requestId}"), onClick = { actions.cancelPrompt(status.requestId) }, enabled = PromptControlPolicy.active(state, status.requestId)) {
                    Icon(Icons.Outlined.Stop, contentDescription = "Cancel message")
                }
            }
        }
    } }
}
