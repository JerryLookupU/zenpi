package zenpi.ui

import androidx.compose.foundation.ScrollState
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.ExperimentalLayoutApi
import androidx.compose.foundation.layout.FlowRow
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowDownward
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.ArrowUpward
import androidx.compose.material.icons.outlined.Keyboard
import androidx.compose.material.icons.outlined.KeyboardReturn
import androidx.compose.material.icons.outlined.KeyboardTab
import androidx.compose.material.icons.outlined.OpenInFull
import androidx.compose.material.icons.outlined.Send
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material.icons.outlined.Terminal
import androidx.compose.material3.Checkbox
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
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
import zenpi.pairing.PairingPhase
import zenpi.pairing.TerminalReadPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.transport.RemoteTerminal

/** Open-terminal entry and the live desktop terminal of the active subtab. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun TerminalSection(scope: ScreenScope, projection: HostProjection, actions: TerminalActions) {
    val state = scope.state
    val selected = scope.selected
    val currentSubtabs = scope.currentSubtabs
    if (state.phase == PairingPhase.CONNECTED && (selected?.terminal == null || selected?.terminal?.running == false) && selected?.terminalOpenControl == true) {
        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
            tooltip = { PlainTooltip { Text("Open desktop terminal") } }, state = rememberTooltipState()) {
            IconButton(onClick = actions::openDesktopTerminal, enabled = !state.scanInProgress && !state.resetting && !state.resetRequired
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
    }?.let { terminal -> LiveTerminal(scope, projection, selected, terminal, actions) }
}

@OptIn(ExperimentalMaterial3Api::class, ExperimentalLayoutApi::class)
@Composable
private fun LiveTerminal(scope: ScreenScope, projection: HostProjection, selected: RemoteProject, terminal: RemoteTerminal, actions: TerminalActions) {
    val state = scope.state
    val hostScope = scope.hostScope
    val sessionScope = scope.sessionScope
    val subtabScope = scope.subtabScope
    val terminalScroll = remember(hostScope, sessionScope, subtabScope, terminal.terminalId) { ScrollState(0) }
    HorizontalDivider()
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        Text("Desktop terminal", modifier = Modifier.weight(1f), style = MaterialTheme.typography.labelMedium)
        if (terminal.terminalId != null) TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
            tooltip = { PlainTooltip { Text("Terminal history") } }, state = rememberTooltipState()) {
            IconButton(onClick = { actions.terminalPage(false) }, enabled = TerminalReadPolicy.browse(state)) {
                Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Read terminal history")
            }
        }
    }
    Box(Modifier.fillMaxWidth().horizontalScroll(terminalScroll).testTag("terminal-viewport")) {
        SelectionContainer {
            Text(terminal.text, modifier = Modifier.testTag("desktop-terminal"), softWrap = false,
                fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
        }
    }
    if (terminal.truncated) Text("Terminal output truncated", style = MaterialTheme.typography.labelSmall)
    if (!terminal.running) Text("Terminal exited", style = MaterialTheme.typography.labelSmall)
    if (selected.terminalControl && terminal.terminalId != null && terminal.running) {
        val enabled = state.pendingWorkspace == null && state.pendingRevoke == null && !state.scanInProgress && !state.resetting && !state.resetRequired
        var input by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId) { mutableStateOf("") }
        var enter by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId) { mutableStateOf(true) }
        var rows by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId, terminal.size) { mutableStateOf((terminal.size?.rows ?: 24).toString()) }
        var cols by remember(hostScope, sessionScope, subtabScope, projection.workspaceOwner, terminal.terminalId, terminal.size) { mutableStateOf((terminal.size?.cols ?: 80).toString()) }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(value = input, onValueChange = { if (it.toByteArray().size <= 4095 && !it.contains('\u0000')) input = it },
                modifier = Modifier.weight(1f).testTag("terminal-input"), label = { Text("Terminal input") }, enabled = enabled)
            TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Send to terminal") } }, state = rememberTooltipState()) {
                IconButton(onClick = { actions.terminalInput(input + if (enter) "\r" else ""); input = "" }, enabled = enabled && input.isNotEmpty()) {
                    Icon(Icons.Outlined.Send, contentDescription = "Send terminal input")
                }
            }
        }
        Row(verticalAlignment = Alignment.CenterVertically) {
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
                    IconButton(onClick = { actions.terminalInput(bytes) }, enabled = enabled) { Icon(icon, contentDescription = name) }
                }
            }
            TextButton(onClick = { actions.terminalInput("\u001b") }, enabled = enabled) { Text("Esc") }
            Box {
                TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Terminal keys") } }, state = rememberTooltipState()) {
                    IconButton(onClick = { keysOpen = true }, enabled = enabled) { Icon(Icons.Outlined.Keyboard, contentDescription = "Terminal keys") }
                }
                DropdownMenu(expanded = keysOpen && enabled, onDismissRequest = { keysOpen = false }) {
                    zenpi.transport.TerminalKey.entries.forEach { key ->
                        DropdownMenuItem(text = { Text(key.label) }, enabled = enabled, onClick = {
                            keysOpen = false; actions.terminalInput(key.bytes)
                        })
                    }
                }
            }
        }
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            OutlinedTextField(rows, { if (it.length <= 3 && it.all(Char::isDigit)) rows = it }, Modifier.weight(1f), label = { Text("Rows") }, singleLine = true, enabled = enabled)
            OutlinedTextField(cols, { if (it.length <= 3 && it.all(Char::isDigit)) cols = it }, Modifier.weight(1f), label = { Text("Columns") }, singleLine = true, enabled = enabled)
            TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(), tooltip = { PlainTooltip { Text("Resize terminal") } }, state = rememberTooltipState()) {
                IconButton(onClick = { actions.terminalResize(rows.toInt(), cols.toInt()) }, enabled = enabled && rows.toIntOrNull() in 1..512 && cols.toIntOrNull() in 1..512) {
                    Icon(Icons.Outlined.OpenInFull, contentDescription = "Resize terminal")
                }
            }
        }
    }
}
