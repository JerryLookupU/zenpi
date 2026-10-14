package zenpi.ui.dialogs

import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.ArrowBack
import androidx.compose.material.icons.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.InsertDriveFile
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.PlainTooltip
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.TooltipBox
import androidx.compose.material3.TooltipDefaults
import androidx.compose.material3.rememberTooltipState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.PairingState
import zenpi.pairing.ResourceReadPolicy
import zenpi.transport.RemoteProject
import zenpi.ui.BrowserActions
import zenpi.ui.Draft

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
