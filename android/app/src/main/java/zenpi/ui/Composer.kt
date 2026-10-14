package zenpi.ui

import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.MenuBook
import androidx.compose.material.icons.automirrored.outlined.Send
import androidx.compose.material.icons.outlined.AttachFile
import androidx.compose.material.icons.outlined.Stop
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.text.input.ImeAction
import zenpi.pairing.FileReadPolicy
import zenpi.pairing.PairingState
import zenpi.pairing.PromptControlPolicy
import zenpi.pairing.ResourceReadPolicy
import zenpi.transport.HostProjection
import zenpi.transport.RemotePromptStatus

private val activePromptStates = setOf("accepted", "running")

/** The composer's three states: send a message, steer the running one, or stop it. */
enum class ComposerMode {
    SEND, STEER, STOP;

    companion object {
        /** Idle sends; while a message runs, typed text steers it and an empty composer stops it. */
        fun of(active: RemotePromptStatus?, text: String): ComposerMode = when {
            active == null -> SEND
            text.isBlank() -> STOP
            else -> STEER
        }

        /** The message the composer steers or stops, if one is running in the selected project. */
        fun active(projection: HostProjection): RemotePromptStatus? =
            projection.projects.singleOrNull { it.id == projection.selectedId }?.promptStatuses?.firstOrNull { it.state in activePromptStates }

        /** Whether the composer's action is available now; the IME Send key uses the same fence. */
        fun enabled(state: PairingState, mode: ComposerMode, active: RemotePromptStatus?, text: String): Boolean = when (mode) {
            SEND -> text.isNotBlank() && !text.trimStart().startsWith("!") && !text.contains('\u0000') && PromptControlPolicy.send(state)
            STEER -> active != null && zenpi.transport.SteeringInput.valid(text) && !text.trimStart().startsWith("!")
                && !text.contains('\u0000') && PromptControlPolicy.active(state, active.requestId)
            STOP -> active != null && PromptControlPolicy.active(state, active.requestId)
        }
    }
}

/**
 * Bottom-pinned message composer: send / steer / stop on one button, `ImeAction.Send` on the
 * keyboard. The draft is the caller's [Draft] (saveable, so it survives rotation).
 */
@Composable
fun Composer(state: PairingState, projection: HostProjection, prompt: Draft, actions: PairingActions) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val active = ComposerMode.active(projection)
    val text = prompt.value
    val mode = ComposerMode.of(active, text)
    val enabled = ComposerMode.enabled(state, mode, active, text)
    val submit = {
        if (ComposerMode.enabled(state, mode, active, text)) when {
            mode == ComposerMode.SEND -> { keyboard?.hide(); focus.clearFocus(); prompt.submit(); actions.prompt(text) }
            active == null -> Unit
            mode == ComposerMode.STEER -> { keyboard?.hide(); focus.clearFocus(); prompt.submit(); actions.steer(active.requestId, text) }
            else -> actions.cancelPrompt(active.requestId)
        }
    }
    Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
        if (active == null) IconButton(onClick = { actions.resources(-1) }, enabled = ResourceReadPolicy.browse(state)) {
            Icon(Icons.AutoMirrored.Outlined.MenuBook, contentDescription = "Select host resource")
        }
        if (active == null && projection.subtabs != null) IconButton(onClick = { actions.files(".") }, enabled = FileReadPolicy.browse(state)) {
            Icon(Icons.Outlined.AttachFile, contentDescription = "Attach host file")
        }
        OutlinedTextField(text, { if (it.toByteArray().size <= 65536) prompt.value = it }, label = { Text(if (active == null) "Message" else "Steer") },
            modifier = Modifier.weight(1f).testTag("prompt-composer"), maxLines = 5,
            enabled = if (active == null) PromptControlPolicy.send(state) else PromptControlPolicy.active(state, active.requestId),
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Send),
            keyboardActions = KeyboardActions(onSend = { if (mode != ComposerMode.STOP) submit() }))
        IconButton(onClick = submit, enabled = enabled, modifier = Modifier.testTag("composer-action")) {
            when (mode) {
                ComposerMode.SEND -> Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = "Send message")
                ComposerMode.STEER -> Icon(Icons.AutoMirrored.Outlined.Send, contentDescription = "Send steering input")
                ComposerMode.STOP -> Icon(Icons.Outlined.Stop, contentDescription = "Stop message")
            }
        }
    }
}
