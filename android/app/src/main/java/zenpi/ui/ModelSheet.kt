package zenpi.ui

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowDropDown
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.ModelReadPolicy
import zenpi.pairing.PairingState
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject

/** Model panel: the host model, reasoning level and model catalog, as a bottom sheet opened from the model chip. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ModelSheet(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: BrowserActions) {
    val hostScope = state.host?.let { it.fingerprint to it.deviceId }
    var modelInput by remember(hostScope, state.modelStatus?.projectId, state.modelStatus?.sessionId, state.modelStatus?.selectionRevision, state.modelOpen) {
        mutableStateOf(state.modelStatus?.model.orEmpty())
    }
    var reasoningMenu by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(false) }
    var modelMenu by remember(state.modelOpen, state.modelStatus?.catalog?.snapshotId) { mutableStateOf(false) }
    var reasoningInput by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(state.modelStatus?.reasoningEffort) }
    ModalBottomSheet(onDismissRequest = actions::closeModel, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp).padding(bottom = 16.dp).testTag("model-sheet")) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("Host model", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            IconButton(onClick = actions::model, enabled = ModelReadPolicy.browse(state)) {
                Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host model")
            }
            IconButton(onClick = actions::closeModel) { Icon(Icons.Outlined.Close, contentDescription = "Close host model") }
        }
        Column(Modifier.fillMaxWidth().verticalScroll(rememberScrollState())) {
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
                    Button(onClick = { actions.selectReasoning(reasoningInput) }, enabled = enabled && reasoningInput != status.reasoningEffort
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
                    Button(onClick = { actions.selectModel(modelInput) }, enabled = enabled && modelInput.isNotBlank()
                        && modelInput != status.model && modelInput.toByteArray().size <= 128) {
                        Icon(Icons.Outlined.Check, contentDescription = null); Spacer(Modifier.width(8.dp)); Text("Apply model")
                    }
                }
                status.catalog?.let { catalog ->
                    val enabled = ModelReadPolicy.navigation(state)
                    Box {
                        OutlinedButton(onClick = { modelMenu = true }, enabled = enabled && catalog.entries.isNotEmpty()) {
                            Text("Model catalog"); Icon(Icons.Outlined.ArrowDropDown, contentDescription = "Select host model from catalog")
                        }
                        DropdownMenu(expanded = modelMenu && enabled, onDismissRequest = { modelMenu = false }) {
                            catalog.entries.forEach { choice -> DropdownMenuItem(enabled = selectionEnabled && selected?.modelControl == true && choice.selectable,
                                text = { Column(Modifier.widthIn(max = 240.dp)) { Text(choice.id, maxLines = 2, overflow = TextOverflow.Ellipsis); Text(choice.status, style = MaterialTheme.typography.labelSmall) } },
                                onClick = { modelInput = choice.id; modelMenu = false }) }
                        }
                    }
                    Row {
                        IconButton(onClick = { actions.modelPage(state.modelPrevious.lastOrNull()?.catalog?.offset ?: (catalog.offset - 16).coerceAtLeast(0)) }, enabled = enabled && state.modelPrevious.isNotEmpty()) {
                            Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous model page")
                        }
                        IconButton(onClick = { actions.modelPage(requireNotNull(catalog.nextOffset)) }, enabled = enabled && catalog.nextOffset != null) {
                            Icon(Icons.Outlined.ArrowForward, contentDescription = "Next model page")
                        }
                    }
                }
                HorizontalDivider()
                if (status.capabilities == null) Text("Capabilities unavailable")
                status.capabilities?.forEach { (name, available) ->
                    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                        Icon(if (available) Icons.Outlined.Check else Icons.Outlined.Close, contentDescription = null)
                        Spacer(Modifier.width(8.dp)); Text(name.replace('_', ' '), modifier = Modifier.weight(1f))
                    }
                }
            }
        } }
    }
}
