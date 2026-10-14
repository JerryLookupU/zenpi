package zenpi.ui

import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.ColumnScope
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.Spacer
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.layout.width
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ArrowForward
import androidx.compose.material.icons.automirrored.outlined.InsertDriveFile
import androidx.compose.material.icons.outlined.Close
import androidx.compose.material.icons.outlined.Folder
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.HorizontalDivider
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.style.TextAlign
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.DiffReadPolicy
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.ModelReadPolicy
import zenpi.pairing.PairingState
import zenpi.pairing.ResourceReadPolicy
import zenpi.pairing.SessionCatalogReadPolicy
import zenpi.pairing.SessionHistoryReadPolicy
import zenpi.transport.ResourceCatalogCodec
import zenpi.transport.WorkspaceFilesCodec

/** Which palette a composer edit opens. */
enum class PaletteTrigger {
    COMMANDS, FILES;

    companion object {
        /**
         * `/` typed into an empty composer opens the command palette; `@` typed at the start of a word opens
         * the file picker. Only a single character appended at the end counts, so pasting or editing never does.
         */
        fun of(before: String, after: String): PaletteTrigger? {
            if (after.length != before.length + 1 || !after.startsWith(before)) return null
            return when (after.last()) {
                '/' -> COMMANDS.takeIf { before.isEmpty() }
                '@' -> FILES.takeIf { before.isEmpty() || before.last().isWhitespace() }
                else -> null
            }
        }

        /** The draft a chosen resource is prepended to: the `/word` that opened the palette is dropped. */
        fun base(draft: String): String {
            val trimmed = draft.trimStart()
            if (!trimmed.startsWith('/')) return draft
            val word = trimmed.takeWhile { !it.isWhitespace() }
            return trimmed.removePrefix(word).trimStart()
        }

        /** Replaces the `@` that opened the picker with [reference], or appends it after a space. */
        fun attach(draft: String, reference: String): String = when {
            draft.endsWith('@') && (draft.length == 1 || draft[draft.length - 2].isWhitespace()) -> draft.dropLast(1) + reference
            draft.isEmpty() || draft.last().isWhitespace() -> draft + reference
            else -> "$draft $reference"
        }
    }
}

/** The phone control that covers a desktop slash command. */
enum class PhoneCommand(val label: String) {
    NEW_SESSION("New session"), MODEL("Model panel"), DIFF("Diff page"), SESSIONS("Sessions page"),
    HISTORY("Conversation history"), ATTACH("File picker"), RELOAD("Reload resources")
}

/**
 * Every desktop slash command (`COMMAND_SPECS` in src/slash.rs, checked by DesktopCommandsTest) and the
 * phone control that covers it. The remote protocol only runs host resources (skills and templates) as
 * slash input, so the others are listed as desktop only rather than sent.
 */
object DesktopCommands {
    private val phone = mapOf("new" to PhoneCommand.NEW_SESSION, "model" to PhoneCommand.MODEL, "models" to PhoneCommand.MODEL,
        "reasoning" to PhoneCommand.MODEL, "diff" to PhoneCommand.DIFF, "session" to PhoneCommand.SESSIONS,
        "history" to PhoneCommand.HISTORY, "attach" to PhoneCommand.ATTACH, "reload" to PhoneCommand.RELOAD)
    val names: List<String> = listOf("new", "yolo", "approval", "reasoning", "persona", "project", "help", "goal", "plan", "model",
        "models", "doctor", "blueprint", "learn", "history", "resources", "layout", "pane", "session", "mailbox", "resume", "recovery",
        "compact", "diff", "attach", "approve", "status", "clear", "cancel", "exit", "compete", "loop", "sync", "execute", "explore",
        "worktree", "tree", "output", "input", "scheduled", "reload", "skills", "templates")
    fun phone(name: String): PhoneCommand? = phone[name]
}

/**
 * The composer's command palette: host resources (skills and templates; choosing one rewrites the draft,
 * never sends) and, while the composer can send, the desktop commands with their phone equivalents or a
 * "Desktop only" mark. Shown while the view model's resource reader is open.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun CommandPalette(scope: ScreenScope, prompt: Draft, workspaceUi: WorkspaceUiState, actions: PairingActions) {
    val state = scope.state
    val selected = scope.selected
    ModalBottomSheet(onDismissRequest = actions::closeResources, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        PaletteFrame("Commands", "Close host resources", actions::closeResources, pager = {
            state.resources?.let { catalog ->
                val enabled = ResourceReadPolicy.navigation(state)
                IconButton(onClick = { actions.resources(state.resourcesPrevious.lastOrNull()?.offset ?: (catalog.offset - 32).coerceAtLeast(0)) },
                    enabled = enabled && state.resourcesPrevious.isNotEmpty()) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Previous resource page")
                }
                Text("${catalog.offset / 32 + 1}", modifier = Modifier.width(24.dp), textAlign = TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                IconButton(onClick = { catalog.nextOffset?.let(actions::resources) }, enabled = enabled && catalog.nextOffset != null) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowForward, contentDescription = "Next resource page")
                }
                if (selected?.resourcesReloadControl == true) IconButton(onClick = actions::reloadResources, enabled = enabled) {
                    Icon(Icons.Outlined.Refresh, contentDescription = "Reload host resources")
                }
            }
        }) {
            val scroll = rememberScrollState()
            LaunchedEffect(state.resources?.snapshotId, state.resources?.generation, state.resources?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().weight(1f, fill = false).verticalScroll(scroll).testTag("resource-catalog-list")) {
                Text("Host resources", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                if (state.resourcesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.resourcesError?.let {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.weight(1f))
                        IconButton(onClick = { actions.resources(-1) }, enabled = ResourceReadPolicy.browse(state)) {
                            Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host resources")
                        }
                    }
                }
                state.resources?.let { catalog ->
                    if (catalog.entries.isEmpty()) Text("No resources", style = MaterialTheme.typography.bodySmall)
                    val base = PaletteTrigger.base(prompt.value)
                    catalog.entries.forEach { resource ->
                        TextButton(onClick = {
                            runCatching { ResourceCatalogCodec.insert(resource, PaletteTrigger.base(prompt.value)) }.getOrNull()?.let {
                                prompt.value = it; actions.closeResources()
                            }
                        }, enabled = ResourceReadPolicy.insertion(state) && resource.invocable && !base.trimStart().startsWith('!')
                            && (resource.command + " " + base).toByteArray().size <= 65536) {
                            Column(Modifier.fillMaxWidth()) {
                                Text(resource.command, maxLines = 2, overflow = TextOverflow.Ellipsis)
                                if (resource.description.isNotEmpty()) Text(resource.description, style = MaterialTheme.typography.bodySmall,
                                    maxLines = 3, overflow = TextOverflow.Ellipsis)
                                if (!resource.invocable) Text("Reload required", style = MaterialTheme.typography.labelSmall)
                            }
                        }
                    }
                }
                if (selected?.promptControl == true) {
                    HorizontalDivider(Modifier.padding(vertical = 8.dp))
                    Text("Desktop commands", style = MaterialTheme.typography.labelLarge, color = MaterialTheme.colorScheme.primary)
                    DesktopCommands.names.forEach { name -> DesktopCommandRow(scope, name, prompt, workspaceUi, actions) }
                }
            }
        }
    }
}

@Composable
private fun DesktopCommandRow(scope: ScreenScope, name: String, prompt: Draft, workspaceUi: WorkspaceUiState, actions: PairingActions) {
    val state = scope.state
    val selected = scope.selected
    val command = DesktopCommands.phone(name)
    val enabled = when (command) {
        null -> false
        PhoneCommand.NEW_SESSION -> selected?.sessionControl == true && zenpi.transport.SessionCommandCodec.mainSelected(state.projection)
        PhoneCommand.MODEL -> ModelReadPolicy.browse(state)
        PhoneCommand.DIFF -> selected?.diffAvailable == true && DiffReadPolicy.browse(state)
        PhoneCommand.SESSIONS -> SessionCatalogReadPolicy.browse(state)
        PhoneCommand.HISTORY -> SessionHistoryReadPolicy.browse(state)
        PhoneCommand.ATTACH -> state.projection?.subtabs != null && FileReadPolicy.browse(state)
        PhoneCommand.RELOAD -> selected?.resourcesReloadControl == true && ResourceReadPolicy.navigation(state)
    }
    TextButton(onClick = {
        // The `/word` that opened the palette is not a message: drop it, then hand over to the phone control.
        prompt.value = PaletteTrigger.base(prompt.value)
        when (command) {
            null -> Unit
            PhoneCommand.NEW_SESSION -> { actions.closeResources(); selected?.let { workspaceUi.newSessionTarget = it.id to it.sessionId } }
            PhoneCommand.MODEL -> actions.model()
            PhoneCommand.DIFF -> actions.diff(".")
            PhoneCommand.SESSIONS -> actions.sessions(-1)
            PhoneCommand.HISTORY -> actions.sessionHistory(false)
            PhoneCommand.ATTACH -> actions.files(".")
            PhoneCommand.RELOAD -> actions.reloadResources()
        }
    }, enabled = enabled, modifier = Modifier.fillMaxWidth().testTag("desktop-command-$name")) {
        Text("/$name", modifier = Modifier.weight(1f), maxLines = 1)
        Text(command?.label ?: "Desktop only", style = MaterialTheme.typography.labelSmall, maxLines = 1, overflow = TextOverflow.Ellipsis)
    }
}

/**
 * The composer's `@` file picker: browse the active worktree, open directories, and choose a file to
 * reference it in the draft (replacing the `@` that opened the picker). Shown while the file reader is open.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun FilePalette(state: PairingState, prompt: Draft, actions: BrowserActions) {
    ModalBottomSheet(onDismissRequest = actions::closeFiles, sheetState = rememberModalBottomSheetState(skipPartiallyExpanded = true)) {
        PaletteFrame("Host files", "Close host files", actions::closeFiles, pager = {
            state.files?.page?.let { page ->
                val enabled = FileReadPolicy.navigation(state)
                IconButton(onClick = { actions.filePage(state.filesPrevious.lastOrNull()?.page?.offset ?: (page.offset - 64).coerceAtLeast(0)) },
                    enabled = enabled && state.filesPrevious.isNotEmpty()) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Previous files page")
                }
                Text("${page.offset / 64 + 1}", modifier = Modifier.width(24.dp), textAlign = TextAlign.Center, style = MaterialTheme.typography.labelMedium)
                IconButton(onClick = { page.nextOffset?.let(actions::filePage) }, enabled = enabled && page.nextOffset != null) {
                    Icon(Icons.AutoMirrored.Outlined.ArrowForward, contentDescription = "Next files page")
                }
            }
        }) {
            val scroll = rememberScrollState()
            LaunchedEffect(state.files?.path, state.files?.page?.snapshotId, state.files?.page?.offset) { scroll.scrollTo(0) }
            Column(Modifier.fillMaxWidth().weight(1f, fill = false).verticalScroll(scroll).testTag("workspace-files-list")) {
                if (state.filesLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
                state.filesError?.let {
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        Text(it, color = MaterialTheme.colorScheme.error, modifier = Modifier.weight(1f))
                        IconButton(onClick = { actions.files(".") }, enabled = FileReadPolicy.browse(state)) {
                            Icon(Icons.Outlined.Refresh, contentDescription = "Refresh host files")
                        }
                    }
                }
                state.files?.let { files ->
                    val enabled = FileReadPolicy.navigation(state)
                    Row(verticalAlignment = Alignment.CenterVertically) {
                        if (files.path != ".") IconButton(onClick = { actions.files(files.path.substringBeforeLast('/', ".")) }, enabled = enabled) {
                            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Parent directory")
                        }
                        Text(files.path, style = MaterialTheme.typography.labelMedium)
                    }
                    if (files.entries.isEmpty()) Text("No files", style = MaterialTheme.typography.bodySmall)
                    files.entries.forEach { file ->
                        TextButton(onClick = {
                            if (file.directory) actions.files(file.path) else {
                                val next = PaletteTrigger.attach(prompt.value, WorkspaceFilesCodec.reference(file))
                                if (next.toByteArray().size <= 65536) { prompt.value = next; actions.closeFiles() }
                            }
                        }, enabled = enabled && (file.directory || file.attachable && FileReadPolicy.attachment(state)), modifier = Modifier.fillMaxWidth()) {
                            Icon(if (file.directory) Icons.Outlined.Folder else Icons.AutoMirrored.Outlined.InsertDriveFile, contentDescription = null)
                            Spacer(Modifier.width(8.dp)); Text(file.name, maxLines = 2, overflow = TextOverflow.Ellipsis, modifier = Modifier.weight(1f))
                        }
                    }
                    if (files.truncated) Text("File list truncated", style = MaterialTheme.typography.labelSmall)
                }
            }
        }
    }
}

/** Sheet frame shared by both palettes: title, pager actions and close above a scrolling body. */
@Composable
private fun PaletteFrame(title: String, close: String, onClose: () -> Unit, pager: @Composable () -> Unit, body: @Composable ColumnScope.() -> Unit) {
    Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp).padding(bottom = 12.dp), verticalArrangement = Arrangement.spacedBy(4.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text(title, style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f))
            pager()
            IconButton(onClick = onClose) { Icon(Icons.Outlined.Close, contentDescription = close) }
        }
        body()
    }
}
