package zenpi.ui

import androidx.compose.material.icons.outlined.History
import androidx.compose.material.icons.automirrored.outlined.OpenInNew
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.BoxWithConstraints
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.KeyboardReturn
import androidx.compose.material.icons.automirrored.outlined.KeyboardTab
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.Keyboard
import androidx.compose.material.icons.outlined.Terminal
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalDensity
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.rememberTextMeasurer
import androidx.compose.ui.unit.dp
import zenpi.pairing.PairingPhase
import zenpi.pairing.TerminalReadPolicy
import zenpi.transport.RemoteTerminal
import zenpi.transport.TerminalSize
import zenpi.ui.nav.SecondaryPage

/** Terminal grid that fits a viewport of monospace cells, within the host's 1..512 bounds. */
object TerminalViewport {
    fun fit(widthPx: Float, heightPx: Float, cellWidthPx: Float, cellHeightPx: Float): TerminalSize? {
        if (cellWidthPx <= 0f || cellHeightPx <= 0f || widthPx <= 0f || heightPx <= 0f) return null
        return TerminalSize((heightPx / cellHeightPx).toInt().coerceIn(1, 512), (widthPx / cellWidthPx).toInt().coerceIn(1, 512))
    }
}

/** The active subtab's desktop terminal, if it is current and readable. */
private fun liveTerminal(scope: ScreenScope): RemoteTerminal? {
    val state = scope.state
    val selected = scope.selected ?: return null
    val projection = state.projection ?: return null
    val tabs = scope.currentSubtabs ?: return null
    if (state.phase != PairingPhase.CONNECTED) return null
    return selected.terminal?.takeIf {
        zenpi.transport.TerminalProjectionCodec.valid(it) && zenpi.transport.SessionCatalogCodec.readable(projection)
            && it.projectId == selected.id && it.sessionId == selected.sessionId && tabs.projectId == selected.id
            && it.subtabId == tabs.activeId && it.subtabRevision == tabs.revision
    }
}

/**
 * Terminal page: the live desktop terminal full screen. Rows and columns are computed from the
 * viewport (recomputed on every size change, so after rotation) and applied with "Resize terminal";
 * the key row and the input sit in the bottom bar, above the keyboard.
 */
@Composable
fun TerminalScreen(scope: ScreenScope, actions: TerminalActions, onBack: () -> Unit) {
    val state = scope.state
    val selected = scope.selected
    val projection = state.projection
    val terminal = liveTerminal(scope)
    val control = selected != null && terminal != null && selected.terminalControl && terminal.terminalId != null && terminal.running
    val enabled = state.pendingWorkspace == null && state.pendingRevoke == null && !state.scanInProgress && !state.resetting && !state.resetRequired
    var fit by remember { mutableStateOf<TerminalSize?>(null) }
    SecondaryPage("Desktop terminal", "Back to conversation", onBack, actions = {
        if (terminal?.terminalId != null) IconButton(onClick = { actions.terminalPage(false) }, enabled = TerminalReadPolicy.browse(state)) {
            Icon(Icons.Outlined.History, contentDescription = "Read terminal history")
        }
        if (state.phase == PairingPhase.CONNECTED && projection != null && (selected?.terminal == null || selected.terminal.running == false)
            && selected?.terminalOpenControl == true) {
            IconButton(onClick = actions::openDesktopTerminal, enabled = enabled
                && runCatching { zenpi.transport.TerminalCommandCodec.open(projection) }.isSuccess) {
                Icon(Icons.AutoMirrored.Outlined.OpenInNew, contentDescription = "Open desktop terminal")
            }
        }
    }, bottomBar = {
        if (control && terminal != null) Surface(tonalElevation = 3.dp) {
            Column(Modifier.navigationBarsPadding().imePadding().padding(horizontal = 8.dp, vertical = 4.dp)) {
                TerminalInput(scope, terminal, enabled, actions)
                TerminalKeys(scope, terminal, enabled, fit, actions)
            }
        }
    }) { padding ->
        Column(Modifier.fillMaxSize().padding(padding).padding(horizontal = 8.dp)) {
            if (terminal == null) {
                Text(if (selected?.terminalOpenControl == true && state.phase == PairingPhase.CONNECTED) "No desktop terminal is open"
                    else "Desktop terminal unavailable", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.padding(8.dp))
                return@Column
            }
            if (terminal.truncated) Text("Terminal output truncated", style = MaterialTheme.typography.labelSmall)
            if (!terminal.running) Text("Terminal exited", style = MaterialTheme.typography.labelSmall)
            TerminalViewportBox(scope, terminal, Modifier.weight(1f)) { fit = it }
        }
    }
}

@Composable
private fun TerminalViewportBox(scope: ScreenScope, terminal: RemoteTerminal, modifier: Modifier, onFit: (TerminalSize?) -> Unit) {
    val style = MaterialTheme.typography.bodySmall.copy(fontFamily = FontFamily.Monospace)
    val measurer = rememberTextMeasurer()
    val cell = remember(style) { measurer.measure("M", style).size }
    val horizontal = remember(scope.hostScope, scope.sessionScope, scope.subtabScope, terminal.terminalId) { ScrollState(0) }
    val vertical = rememberScrollState()
    BoxWithConstraints(modifier.fillMaxWidth()) {
        val density = LocalDensity.current
        val size = with(density) { TerminalViewport.fit(maxWidth.toPx(), maxHeight.toPx(), cell.width.toFloat(), cell.height.toFloat()) }
        LaunchedEffect(size) { onFit(size) }
        // Follow the newest output, as a terminal does.
        LaunchedEffect(terminal.text) { vertical.scrollTo(vertical.maxValue) }
        Box(Modifier.fillMaxSize().verticalScroll(vertical)) {
            Box(Modifier.fillMaxWidth().horizontalScroll(horizontal).testTag("terminal-viewport")) {
                SelectionContainer {
                    Text(terminal.text, modifier = Modifier.testTag("desktop-terminal"), softWrap = false, style = style)
                }
            }
        }
    }
}

@Composable
private fun TerminalInput(scope: ScreenScope, terminal: RemoteTerminal, enabled: Boolean, actions: TerminalActions) {
    val owner = scope.state.projection?.workspaceOwner
    var input by remember(scope.hostScope, scope.sessionScope, scope.subtabScope, owner, terminal.terminalId) { mutableStateOf("") }
    var enter by remember(scope.hostScope, scope.sessionScope, scope.subtabScope, owner, terminal.terminalId) { mutableStateOf(true) }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Checkbox(enter, { enter = it }, enabled = enabled, modifier = Modifier.semantics { contentDescription = "Append Enter" })
        OutlinedTextField(value = input, onValueChange = { if (it.toByteArray().size <= 4095 && !it.contains('\u0000')) input = it },
            modifier = Modifier.weight(1f).testTag("terminal-input"), label = { Text("Terminal input") }, enabled = enabled, maxLines = 3)
        IconButton(onClick = { actions.terminalInput(input + if (enter) "\r" else ""); input = "" }, enabled = enabled && input.isNotEmpty()) {
            Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = "Send terminal input")
        }
    }
}

/** The key row above the keyboard: control keys, the extra-keys menu and fit-to-screen. */
@Composable
private fun TerminalKeys(scope: ScreenScope, terminal: RemoteTerminal, enabled: Boolean, fit: TerminalSize?, actions: TerminalActions) {
    var keysOpen by remember(scope.hostScope, scope.sessionScope, scope.subtabScope, scope.state.projection?.workspaceOwner, terminal.terminalId) { mutableStateOf(false) }
    LaunchedEffect(enabled) { if (!enabled) keysOpen = false }
    Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).testTag("terminal-keys"), verticalAlignment = Alignment.CenterVertically,
        horizontalArrangement = Arrangement.spacedBy(0.dp)) {
        TextButton(onClick = { actions.terminalInput("\u001b") }, enabled = enabled) { Text("Esc") }
        IconButton(onClick = { actions.terminalInput("\t") }, enabled = enabled) {
            Icon(Icons.AutoMirrored.Outlined.KeyboardTab, contentDescription = "Tab")
        }
        // Keys drawn as keycap text, so their glyphs never double as the app's stop, expand or paging icons.
        listOf(Triple("Interrupt terminal", "\u0003", "^C"), Triple("Up", "\u001b[A", "\u2191"), Triple("Down", "\u001b[B", "\u2193"),
            Triple("Left", "\u001b[D", "\u2190"), Triple("Right", "\u001b[C", "\u2192")).forEach { (name, bytes, cap) ->
            IconButton(onClick = { actions.terminalInput(bytes) }, enabled = enabled, modifier = Modifier.semantics { contentDescription = name }) {
                Text(cap, style = MaterialTheme.typography.titleLarge, modifier = Modifier.clearAndSetSemantics {})
            }
        }
        IconButton(onClick = { actions.terminalInput("\r") }, enabled = enabled) {
            Icon(Icons.AutoMirrored.Outlined.KeyboardReturn, contentDescription = "Enter")
        }
        Box {
            IconButton(onClick = { keysOpen = true }, enabled = enabled) { Icon(Icons.Outlined.Keyboard, contentDescription = "Terminal keys") }
            DropdownMenu(expanded = keysOpen && enabled, onDismissRequest = { keysOpen = false }) {
                zenpi.transport.TerminalKey.entries.forEach { key ->
                    DropdownMenuItem(text = { Text(key.label) }, enabled = enabled, onClick = { keysOpen = false; actions.terminalInput(key.bytes) })
                }
            }
        }
        TextButton(onClick = { fit?.let { actions.terminalResize(it.rows, it.cols) } }, enabled = enabled && fit != null,
            modifier = Modifier.semantics { contentDescription = "Resize terminal" }.testTag("terminal-fit")) {
            Text(fit?.let { "${it.rows}×${it.cols}" } ?: "Fit")
        }
    }
}
