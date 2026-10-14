package zenpi.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxSize
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.imePadding
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.foundation.text.selection.SelectionContainer
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material.icons.automirrored.outlined.ArrowForward
import androidx.compose.material.icons.outlined.Refresh
import androidx.compose.material3.FilterChip
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.LinearProgressIndicator
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.setValue
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.unit.dp
import zenpi.pairing.DiffReadPolicy
import zenpi.transport.DiffProjectionCodec
import zenpi.ui.nav.SecondaryPage

/** Changed paths named by a `git status --porcelain` listing; renames contribute their new path. */
object DiffFiles {
    fun of(status: String): List<String> = status.lineSequence().filter { it.length > 3 && it[2] == ' ' }
        .map { it.substring(3).substringAfter(" -> ").trim().removeSurrounding("\"") }
        .filter { it.isNotEmpty() && !it.endsWith('/') }.distinct().toList()
}

/**
 * Diff page: full screen and per file. The changed files of the worktree are chips that load that
 * file's diff; the patch is coloured per line (+ / - / @@). The conversation's scroll position is kept
 * by the route host, so going back lands where the reader left it.
 */
@Composable
fun DiffScreen(scope: ScreenScope, actions: BrowserActions, onBack: () -> Unit) {
    val state = scope.state
    var path by rememberSaveable(scope.hostScope, scope.sessionScope, state.projection?.subtabs?.activeId) { mutableStateOf(".") }
    val diffEnabled = DiffReadPolicy.browse(state)
    val focus = LocalFocusManager.current
    val keyboard = LocalSoftwareKeyboardController.current
    val load = { target: String -> focus.clearFocus(); keyboard?.hide(); path = target; actions.diff(target) }
    SecondaryPage("Diff", "Close diff", onBack, actions = {
        val navigation = DiffReadPolicy.navigation(state)
        val previous = state.diff?.let { runCatching { DiffProjectionCodec.previous(state.diffPrevious, it) }.getOrNull() }
        IconButton(onClick = actions::previousDiff, enabled = navigation && previous != null) {
            Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = "Previous diff page")
        }
        IconButton(onClick = actions::nextDiff, enabled = navigation && state.diff?.page?.hasMore == true) {
            Icon(Icons.AutoMirrored.Outlined.ArrowForward, contentDescription = "Next diff page")
        }
        IconButton(onClick = { load(path) }, enabled = diffEnabled && path.isNotBlank()) {
            Icon(Icons.Outlined.Refresh, contentDescription = "Refresh diff")
        }
    }) { padding ->
        val scroll = rememberScrollState()
        LaunchedEffect(state.diff?.path, state.diff?.page?.offset) { scroll.scrollTo(0) }
        Column(Modifier.fillMaxSize().padding(padding).navigationBarsPadding().imePadding().verticalScroll(scroll).padding(16.dp),
            verticalArrangement = Arrangement.spacedBy(8.dp)) {
            OutlinedTextField(path, { if (it.toByteArray().size <= 4096) path = it }, label = { Text("Path") },
                modifier = Modifier.fillMaxWidth(), singleLine = true, enabled = diffEnabled,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done),
                keyboardActions = KeyboardActions(onDone = { if (diffEnabled) load(path) else { focus.clearFocus(); keyboard?.hide() } }))
            if (state.diffLoading) LinearProgressIndicator(Modifier.fillMaxWidth())
            state.diffError?.let { Text(it, color = MaterialTheme.colorScheme.error) }
            state.diff?.let { diff ->
                val files = DiffFiles.of(diff.status)
                if (files.isNotEmpty()) Row(Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).testTag("diff-files"),
                    horizontalArrangement = Arrangement.spacedBy(8.dp)) {
                    FilterChip(selected = diff.path == ".", onClick = { load(".") }, enabled = diffEnabled, label = { Text("All changes") })
                    files.forEach { file ->
                        FilterChip(selected = diff.path == file, onClick = { load(file) }, enabled = diffEnabled,
                            label = { Text(file, maxLines = 1, overflow = TextOverflow.Ellipsis) })
                    }
                }
                Text(diff.path, style = MaterialTheme.typography.labelLarge)
                Text(when { diff.binary -> "Binary file"; !diff.changed -> "No changes"; diff.truncated -> "Diff truncated"; else -> "Changes" },
                    style = MaterialTheme.typography.labelMedium)
                diff.page?.let { Text("${it.offset} - ${it.nextOffset} / ${it.totalBytes} bytes", style = MaterialTheme.typography.labelSmall) }
                if (diff.status.isNotEmpty()) Text(diff.status, fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall)
                SelectionContainer {
                    Text(patchText(diff.diff), modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).testTag("diff-patch"),
                        fontFamily = FontFamily.Monospace, style = MaterialTheme.typography.bodySmall, softWrap = false)
                }
            }
        }
    }
}
