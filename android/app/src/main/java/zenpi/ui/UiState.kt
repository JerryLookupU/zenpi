package zenpi.ui

import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.MutableState
import androidx.compose.runtime.State
import androidx.compose.runtime.Stable
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.produceState
import androidx.compose.runtime.remember
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import kotlinx.coroutines.delay
import zenpi.pairing.PairingInvite

/** Project-level dialogs; discarded when the host or session changes. */
@Stable
class WorkspaceUiState {
    var openWorkspace by mutableStateOf(false)
    var closeProject by mutableStateOf<String?>(null)
    var newSessionTarget by mutableStateOf<Pair<String, String>?>(null)
    var renameSessionTarget by mutableStateOf<Pair<String, String>?>(null)
    var sessionNameDraft by mutableStateOf("")
}

/** Subtab dialogs; discarded when the host, session or subtab projection changes. */
@Stable
class SubtabUiState {
    var renameSubtab by mutableStateOf<String?>(null)
    var subtabName by mutableStateOf("")
    var directoryTabDialog by mutableStateOf(false)
    var directoryTabName by mutableStateOf("")
    var directoryTabPath by mutableStateOf("")
    var closeDirectoryTab by mutableStateOf<String?>(null)
    var worktreeDialog by mutableStateOf(false)
    var worktreeName by mutableStateOf("")
    var closeWorktree by mutableStateOf<String?>(null)
}

/** A composer draft, see [DraftKey] and [DraftPolicy]. */
@Stable
class Draft internal constructor(private val text: MutableState<String>, private val submitted: MutableState<String?>) {
    var value: String
        get() = text.value
        set(next) { text.value = next }

    /** Marks the current text as sent; it is cleared once the host acknowledges it. */
    fun submit() { submitted.value = text.value }

    internal fun settle(previous: Any?, current: Any?) {
        text.value = DraftPolicy.settle(text.value, submitted.value, previous, current)
        submitted.value = null
    }
}

@Composable
fun rememberDraft(key: Any, marker: Any?): Draft {
    val text = rememberSaveable(key) { mutableStateOf("") }
    val submitted = rememberSaveable(key) { mutableStateOf<String?>(null) }
    val draft = remember(key) { Draft(text, submitted) }
    val seen = remember(key) { mutableStateOf(marker) }
    LaunchedEffect(key, marker) {
        if (seen.value != marker) { draft.settle(seen.value, marker); seen.value = marker }
    }
    return draft
}

/** Wall-clock seconds for the invitation countdown; static while no invitation is live. */
@Composable
fun rememberInviteClock(invite: PairingInvite?): State<Long> =
    produceState(System.currentTimeMillis() / 1000, invite) {
        InviteTicker.run(active = { InviteTicker.active(invite, it) }, clock = { System.currentTimeMillis() / 1000 },
            sleep = { delay(it) }, publish = { value = it })
    }
