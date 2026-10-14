package zenpi.ui.dialogs

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.layout.widthIn
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowDropDown
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Check
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.InsertDriveFile
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.Button
import androidx.compose.material3.DropdownMenu
import androidx.compose.material3.DropdownMenuItem
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedButton
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
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.DiffReadPolicy
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.ModelReadPolicy
import zenpi.pairing.PairingState
import zenpi.pairing.ResourceReadPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemoteProject
import zenpi.ui.BrowserActions
import zenpi.ui.Draft
import zenpi.ui.ScreenScope

/** Host model, reasoning level and model catalog. */
@Composable
fun ModelDialog(state: PairingState, projection: HostProjection, selected: RemoteProject?, actions: BrowserActions) {
    val hostScope = state.host?.let { it.fingerprint to it.deviceId }
    var modelInput by remember(hostScope, state.modelStatus?.projectId, state.modelStatus?.sessionId, state.modelStatus?.selectionRevision, state.modelOpen) {
        mutableStateOf(state.modelStatus?.model.orEmpty())
    }
    var reasoningMenu by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(false) }
    var modelMenu by remember(state.modelOpen, state.modelStatus?.catalog?.snapshotId) { mutableStateOf(false) }
    var reasoningInput by remember(state.modelOpen, state.modelStatus?.selectionRevision) { mutableStateOf(state.modelStatus?.reasoningEffort) }
    AlertDialog(onDismissRequest = actions::closeModel, title = { Text("Host model") },
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
        } }, confirmButton = { Row {
            IconButton(onClick = actions::model, enabled = ModelReadPolicy.browse(state)) {
                Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host model")
            }
            IconButton(onClick = actions::closeModel) { Icon(Icons.Outlined.Close, contentDescription = "Close host model") }
        } })
}

/** Host resources (slash commands); choosing one rewrites the message draft. */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ResourcesDialog(state: PairingState, selected: RemoteProject?, prompt: Draft, actions: BrowserActions) {
    AlertDialog(onDismissRequest = actions::closeResources, title = { Text("Host resources") },
        text = {
            val scroll = rememberScrollState()
            LaunchedEffect(state.resources?.snapshotId, state.resources?.generation, state.resources?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll).testTag("resource-catalog-list")) {
                val resourcesEnabled = ResourceReadPolicy.browse(state)
                if (state.resourcesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.resourcesError?.let { Text(it, color = MaterialTheme.colorScheme.error)
                    IconButton(onClick = { actions.resources(-1) }, enabled = resourcesEnabled) {
                        Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host resources")
                    }
                }
                state.resources?.let { catalog ->
                    val selectionEnabled = ResourceReadPolicy.navigation(state)
                    if (selected?.resourcesReloadControl == true) {
                        TooltipBox(positionProvider = TooltipDefaults.rememberPlainTooltipPositionProvider(),
                            tooltip = { PlainTooltip { Text("Reload host resources") } }, state = rememberTooltipState()) {
                            IconButton(onClick = actions::reloadResources, enabled = selectionEnabled) {
                                Icon(Icons.Outlined.Refresh, contentDescription = "Reload host resources")
                            }
                        }
                    }
                    if (catalog.entries.isEmpty()) Text("No resources")
                    val text = prompt.value
                    catalog.entries.forEach { resource ->
                        TextButton(onClick = {
                            runCatching { zenpi.transport.ResourceCatalogCodec.insert(resource, prompt.value) }.getOrNull()?.let {
                                prompt.value = it; actions.closeResources()
                            }
                        }, enabled = ResourceReadPolicy.insertion(state) && resource.invocable && !text.trimStart().startsWith('/')
                            && !text.trimStart().startsWith('!') && (resource.command + " " + text).toByteArray().size <= 65536) {
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
                Row(verticalAlignment = Alignment.CenterVertically) {
                    IconButton(onClick = { actions.resources(state.resourcesPrevious.lastOrNull()?.offset ?: (catalog.offset - 32).coerceAtLeast(0)) }, enabled = selectionEnabled && state.resourcesPrevious.isNotEmpty()) {
                        Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous resource page")
                    }
                    Text("${catalog.offset / 32 + 1}", modifier = Modifier.width(32.dp),
                        textAlign = TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                    IconButton(onClick = { catalog.nextOffset?.let(actions::resources) }, enabled = selectionEnabled && catalog.nextOffset != null) {
                        Icon(Icons.Outlined.ArrowForward, contentDescription = "Next resource page")
                    }
                }
            }
        }, dismissButton = { IconButton(onClick = actions::closeResources) { Icon(Icons.Outlined.Close, contentDescription = "Close host resources") } })
}

/** Host file browser; choosing a file appends an `@` reference to the message draft. */
@Composable
fun FilesDialog(state: PairingState, prompt: Draft, actions: BrowserActions) {
    AlertDialog(onDismissRequest = actions::closeFiles, title = { Text("Host files") },
        text = {
            val scroll = rememberScrollState()
            LaunchedEffect(state.files?.path, state.files?.page?.snapshotId, state.files?.page?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().heightIn(max = 420.dp).verticalScroll(scroll).testTag("workspace-files-list")) {
                val filesEnabled = FileReadPolicy.browse(state)
                if (state.filesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.filesError?.let { Text(it, color = MaterialTheme.colorScheme.error)
                    IconButton(onClick = { actions.files(".") }, enabled = filesEnabled) {
                        Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host files")
                    }
                }
                state.files?.let { files ->
                    val selectionEnabled = FileReadPolicy.navigation(state)
                    Text(files.path, style = MaterialTheme.typography.labelMedium)
                    if (files.path != ".") IconButton(onClick = { actions.files(files.path.substringBeforeLast('/', ".")) }, enabled = selectionEnabled) {
                        Icon(Icons.Outlined.ArrowBack, contentDescription = "Parent directory")
                    }
                    if (files.entries.isEmpty()) Text("No files")
                    files.entries.forEach { file ->
                        TextButton(onClick = {
                            if (file.directory) actions.files(file.path) else {
                                val reference = zenpi.transport.WorkspaceFilesCodec.reference(file)
                                val current = prompt.value
                                val next = current + (if (current.isEmpty() || current.last().isWhitespace()) "" else " ") + reference
                                if (next.toByteArray().size <= 65536) { prompt.value = next; actions.closeFiles() }
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
                Row(verticalAlignment = Alignment.CenterVertically) {
                    IconButton(onClick = { actions.filePage(state.filesPrevious.lastOrNull()?.page?.offset ?: (page.offset - 64).coerceAtLeast(0)) }, enabled = selectionEnabled && state.filesPrevious.isNotEmpty()) {
                        Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous files page")
                    }
                    Text("${page.offset / 64 + 1}", modifier = Modifier.width(32.dp),
                        textAlign = TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                    IconButton(onClick = { page.nextOffset?.let(actions::filePage) }, enabled = selectionEnabled && page.nextOffset != null) {
                        Icon(Icons.Outlined.ArrowForward, contentDescription = "Next files page")
                    }
                }
            }
        }, dismissButton = { IconButton(onClick = actions::closeFiles) { Icon(Icons.Outlined.Close, contentDescription = "Close host files") } })
}

/** Paged diff review for a path of the selected workspace. */
@Composable
fun DiffDialog(state: PairingState, scope: ScreenScope, actions: BrowserActions) {
    var path by rememberSaveable(scope.hostScope, scope.sessionScope, state.projection?.subtabs?.activeId) { mutableStateOf(".") }
    val diffEnabled = DiffReadPolicy.browse(state)
    AlertDialog(onDismissRequest = actions::closeDiff, title = { Text("Diff") },
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
                    keyboardActions = KeyboardActions(onDone = { dialogFocus.clearFocus(); dialogKeyboard?.hide(); if (diffEnabled) actions.diff(path) }))
                if (state.diffLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.diffError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
                state.diff?.let { diff ->
                    Text(diff.path, style = MaterialTheme.typography.labelLarge)
                    Text(when { diff.binary -> "Binary file"; !diff.changed -> "No changes"; diff.truncated -> "Diff truncated"; else -> "Changes" },
                        style = MaterialTheme.typography.labelMedium)
                    diff.page?.let { Text("${it.offset} - ${it.nextOffset} / ${it.totalBytes} bytes", style = MaterialTheme.typography.labelSmall) }
                    SelectionContainer {
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
            IconButton(onClick = actions::previousDiff, enabled = navigation && previous != null) { Icon(Icons.Outlined.ArrowBack, contentDescription = "Previous diff page") }
            IconButton(onClick = actions::nextDiff, enabled = navigation && state.diff?.page?.hasMore == true) { Icon(Icons.Outlined.ArrowForward, contentDescription = "Next diff page") }
            IconButton(onClick = { dialogFocus.clearFocus(); dialogKeyboard?.hide(); actions.diff(path) }, enabled = diffEnabled && path.isNotBlank()) {
                Icon(Icons.Outlined.Refresh, contentDescription = "Refresh diff") } } },
        dismissButton = { IconButton(onClick = actions::closeDiff) { Icon(Icons.Outlined.Close, contentDescription = "Close diff") } })
}
