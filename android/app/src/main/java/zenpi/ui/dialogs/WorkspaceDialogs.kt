package zenpi.ui.dialogs

import zenpi.ui.theme.asHeading
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.text.KeyboardActions
import androidx.compose.foundation.text.KeyboardOptions
import androidx.compose.material3.AlertDialog
import androidx.compose.material3.OutlinedTextField
import androidx.compose.material3.Text
import androidx.compose.material3.TextButton
import androidx.compose.runtime.Composable
import androidx.compose.runtime.MutableState
import androidx.compose.ui.platform.LocalFocusManager
import androidx.compose.ui.platform.LocalSoftwareKeyboardController
import androidx.compose.ui.text.input.ImeAction
import androidx.compose.ui.unit.dp
import zenpi.pairing.DeviceActionPolicy
import zenpi.pairing.PairingState
import zenpi.ui.ConnectionActions
import zenpi.ui.ScreenScope
import zenpi.ui.SubtabUiState
import zenpi.ui.WorkspaceActions
import zenpi.ui.WorkspaceUiState

private val worktreeBranch = Regex("[A-Za-z0-9_][A-Za-z0-9_-]{0,127}")

/** Open / close project workspace confirmations. */
@Composable
fun WorkspaceDialogs(state: PairingState, scope: ScreenScope, workspaceUi: WorkspaceUiState, directory: MutableState<String>, actions: WorkspaceActions) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    if (workspaceUi.openWorkspace) AlertDialog(onDismissRequest = { workspaceUi.openWorkspace = false }, title = { Text("Open workspace", Modifier.asHeading()) },
        text = { OutlinedTextField(directory.value, { directory.value = it }, label = { Text("Host directory") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() })) },
        confirmButton = { TextButton(enabled = scope.workspaceEnabled && directory.value.isNotBlank(), onClick = {
            state.projection?.selectedId?.let { actions.workspace("workspace_open", it, directory.value) }; workspaceUi.openWorkspace = false
        }) { Text("Open") } }, dismissButton = { TextButton(onClick = { workspaceUi.openWorkspace = false }) { Text("Cancel") } })
    workspaceUi.closeProject?.let { id -> AlertDialog(onDismissRequest = { workspaceUi.closeProject = null }, title = { Text("Close workspace?", Modifier.asHeading()) },
        confirmButton = { TextButton(enabled = scope.workspaceEnabled && state.projection?.selectedId == id && (state.projection?.projects?.size ?: 0) > 1,
            onClick = { actions.workspace("workspace_close", id, null); workspaceUi.closeProject = null }) { Text("Close") } },
        dismissButton = { TextButton(onClick = { workspaceUi.closeProject = null }) { Text("Cancel") } }) }
}

/** Rename / add / remove worktree and directory subtabs. */
@Composable
fun SubtabDialogs(state: PairingState, scope: ScreenScope, subtabUi: SubtabUiState, actions: WorkspaceActions) {
    val keyboard = LocalSoftwareKeyboardController.current
    val focus = LocalFocusManager.current
    val subtabEnabled = scope.subtabEnabled
    subtabUi.renameSubtab?.let { id -> AlertDialog(onDismissRequest = { subtabUi.renameSubtab = null }, title = { Text("Rename worktree tab", Modifier.asHeading()) },
        text = { OutlinedTextField(subtabUi.subtabName, { subtabUi.subtabName = it }, label = { Text("Tab name") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() })) },
        confirmButton = { TextButton(enabled = subtabUi.subtabName.trim().isNotEmpty() && subtabUi.subtabName.toByteArray().size <= 128
            && subtabEnabled,
            onClick = { actions.subtab("subtab_rename", id, subtabUi.subtabName.trim()); subtabUi.renameSubtab = null }) { Text("Rename") } },
        dismissButton = { TextButton(onClick = { subtabUi.renameSubtab = null }) { Text("Cancel") } }) }
    if (subtabUi.directoryTabDialog) AlertDialog(onDismissRequest = { subtabUi.directoryTabDialog = false }, title = { Text("Add directory tab", Modifier.asHeading()) },
        text = { Column(verticalArrangement = Arrangement.spacedBy(12.dp)) {
            OutlinedTextField(subtabUi.directoryTabName, { subtabUi.directoryTabName = it }, label = { Text("Directory tab name") }, singleLine = true)
            OutlinedTextField(subtabUi.directoryTabPath, { subtabUi.directoryTabPath = it }, label = { Text("Project subdirectory") }, singleLine = true,
                keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { keyboard?.hide(); focus.clearFocus() }))
        } }, confirmButton = { TextButton(enabled = subtabUi.directoryTabName.trim().isNotEmpty() && subtabUi.directoryTabPath.isNotBlank()
            && subtabEnabled && state.projection?.subtabs?.directoryControl == true,
            onClick = { actions.addDirectoryTab(subtabUi.directoryTabName.trim(), subtabUi.directoryTabPath); subtabUi.directoryTabDialog = false }) { Text("Add") } },
        dismissButton = { TextButton(onClick = { subtabUi.directoryTabDialog = false }) { Text("Cancel") } })
    if (subtabUi.worktreeDialog) AlertDialog(onDismissRequest = { subtabUi.worktreeDialog = false }, title = { Text("Add Git worktree", Modifier.asHeading()) },
        text = { OutlinedTextField(subtabUi.worktreeName, { subtabUi.worktreeName = it }, label = { Text("New branch name") }, singleLine = true,
            keyboardOptions = KeyboardOptions(imeAction = ImeAction.Done), keyboardActions = KeyboardActions(onDone = { focus.clearFocus(); keyboard?.hide() })) },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.worktreeControl == true
            && subtabUi.worktreeName.matches(worktreeBranch), onClick = {
            actions.addWorktreeTab(subtabUi.worktreeName); subtabUi.worktreeDialog = false
        }) { Text("Create") } }, dismissButton = { TextButton(onClick = { subtabUi.worktreeDialog = false }) { Text("Cancel") } })
    subtabUi.closeWorktree?.let { id -> AlertDialog(onDismissRequest = { subtabUi.closeWorktree = null }, title = { Text("Remove Git worktree?", Modifier.asHeading()) },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.worktreeControl == true,
            onClick = { actions.subtab("subtab_close", id, null); subtabUi.closeWorktree = null }) { Text("Remove") } },
        dismissButton = { TextButton(onClick = { subtabUi.closeWorktree = null }) { Text("Cancel") } }) }
    subtabUi.closeDirectoryTab?.let { id -> AlertDialog(onDismissRequest = { subtabUi.closeDirectoryTab = null }, title = { Text("Close directory tab?", Modifier.asHeading()) },
        confirmButton = { TextButton(enabled = subtabEnabled && state.projection?.subtabs?.directoryControl == true,
            onClick = { actions.subtab("subtab_close", id, null); subtabUi.closeDirectoryTab = null }) { Text("Close") } },
        dismissButton = { TextButton(onClick = { subtabUi.closeDirectoryTab = null }) { Text("Cancel") } }) }
}

/** Revoke-on-host and forget-locally confirmations. */
object DeviceDialogs {
    @Composable
    fun Revoke(state: PairingState, open: MutableState<Boolean>, actions: ConnectionActions) {
        AlertDialog(onDismissRequest = { open.value = false }, title = { Text("Revoke this device?", Modifier.asHeading()) },
            text = { Text("This device will lose access to the host. Local keys are removed only after host confirmation.") },
            confirmButton = { TextButton(enabled = DeviceActionPolicy.revoke(state),
                onClick = { open.value = false; actions.revoke() }) { Text("Revoke") } },
            dismissButton = { TextButton(onClick = { open.value = false }) { Text("Cancel") } })
    }

    @Composable
    fun Forget(state: PairingState, open: MutableState<Boolean>, actions: ConnectionActions) {
        AlertDialog(onDismissRequest = { open.value = false }, title = { Text("Remove local device data?", Modifier.asHeading()) },
            text = { Text("Local keys, cached messages and unresolved request records will be removed. This does not revoke access on the desktop.") },
            confirmButton = { TextButton(enabled = DeviceActionPolicy.forget(state),
                onClick = { open.value = false; actions.forget() }) { Text("Remove local data") } },
            dismissButton = { TextButton(onClick = { open.value = false }) { Text("Cancel") } })
    }
}
