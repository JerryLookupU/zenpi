package zenpi.ui

import android.content.Context
import android.hardware.biometrics.BiometricManager
import android.hardware.biometrics.BiometricPrompt
import android.os.CancellationSignal
import androidx.compose.foundation.clickable
import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.heightIn
import androidx.compose.foundation.layout.navigationBarsPadding
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.foundation.verticalScroll
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.outlined.TaskAlt
import androidx.compose.material.icons.outlined.Block
import androidx.compose.material.icons.outlined.WarningAmber
import androidx.compose.material3.Button
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.ModalBottomSheet
import androidx.compose.material3.ModalBottomSheetProperties
import androidx.compose.material3.OutlinedButton
import androidx.compose.material3.SheetValue
import androidx.compose.material3.Surface
import androidx.compose.material3.Switch
import androidx.compose.material3.Text
import androidx.compose.material3.rememberModalBottomSheetState
import androidx.compose.runtime.Composable
import androidx.compose.runtime.LaunchedEffect
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateListOf
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.remember
import androidx.compose.runtime.rememberUpdatedState
import androidx.compose.runtime.saveable.rememberSaveable
import androidx.compose.runtime.setValue
import androidx.compose.ui.Alignment
import androidx.compose.ui.Modifier
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.graphics.luminance
import androidx.compose.ui.platform.LocalContext
import androidx.compose.ui.platform.testTag
import androidx.compose.ui.semantics.Role
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.semantics
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.style.TextOverflow
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp
import zenpi.pairing.ApprovalDecisionPolicy
import zenpi.pairing.PairingState
import zenpi.transport.RemoteApproval
import zenpi.ui.theme.asHeading
import zenpi.ui.theme.spokenAs

private fun sideEffectLabel(approval: RemoteApproval): String = when (approval.sideEffect) {
    "workspace_write" -> "Workspace write"; "command_execution" -> "Command execution"; else -> "Read only"
}

private fun key(approval: RemoteApproval): String = "${approval.workerSessionId}/${approval.requestId}"

/** Test tag of the bar's decision buttons; it carries the ids, which TalkBack never reads. */
private fun tag(decision: String, approval: RemoteApproval): String =
    "$decision approval ${approval.requestId}" + (approval.workerSessionId?.let { " for worker $it" } ?: "")

private fun spoken(decision: String, approval: RemoteApproval): String =
    "$decision approval" + (if (approval.workerSessionId != null) " for worktree task" else "")

/**
 * The approval bar, pinned above the composer so it stays visible wherever the conversation is
 * scrolled, and its bottom sheet. The bar shows the head of the selected project's queue with one-tap
 * Deny / Allow; tapping it opens the sheet with i/N and the coloured patch. While the queue holds a
 * decision this device may make, the sheet cannot be dismissed: it closes when the queue drains.
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun ApprovalPanel(scope: ScreenScope, actions: ApprovalActions) {
    val state = scope.state
    val queue = ApprovalDecisionPolicy.queue(state)
    val control = scope.selected?.approvalControl == true
    var open by rememberSaveable(scope.hostScope, scope.sessionScope) { mutableStateOf(false) }
    // Approvals answered from this sheet that have left the queue: the "i" of i/N.
    val answered = remember(scope.hostScope, scope.sessionScope) { mutableStateListOf<String>() }
    val confirmation = rememberApprovalConfirmation()
    val latest by rememberUpdatedState(state)
    LaunchedEffect(queue.isEmpty()) { if (queue.isEmpty()) { open = false; answered.clear() } }
    val head = queue.firstOrNull() ?: return
    val decide = { target: RemoteApproval, decision: String ->
        val commit = {
            // Re-checked when the (possibly biometric) confirmation lands: the head and its digest must still match.
            if (ApprovalDecisionPolicy.decides(latest, target)) { answered += key(target); actions.approve(target, decision) }
        }
        if (ApprovalDecisionPolicy.decides(state, target)) {
            if (confirmation.enabled) confirmation.confirm(if (decision == "allow") "Allow ${target.tool} once" else "Deny ${target.tool}", commit)
            else commit()
        }
    }
    ApprovalBar(state, head, queue.size, control, onReview = { open = true }, decide = decide)
    if (open) {
        val keys = queue.map(::key).toSet()
        val done = answered.count { it !in keys }
        val locked = control
        val sheet = rememberModalBottomSheetState(skipPartiallyExpanded = true, confirmValueChange = { it != SheetValue.Hidden || !locked })
        ModalBottomSheet(onDismissRequest = { if (!locked) open = false }, sheetState = sheet,
            properties = ModalBottomSheetProperties(shouldDismissOnBackPress = !locked)) {
            ApprovalSheetContent(state, head, done + 1, done + queue.size, control, confirmation, decide)
        }
    }
}

/** One line per queue: the head's tool and effect, the queue length, and one-tap decisions. */
@Composable
private fun ApprovalBar(state: PairingState, head: RemoteApproval, pending: Int, control: Boolean, onReview: () -> Unit,
    decide: (RemoteApproval, String) -> Unit) {
    Surface(color = MaterialTheme.colorScheme.secondaryContainer, modifier = Modifier.testTag("approval-bar")) {
        Row(Modifier.fillMaxWidth().padding(start = 12.dp), verticalAlignment = Alignment.CenterVertically) {
            Row(Modifier.weight(1f).clickable(role = Role.Button, onClickLabel = "Review approvals", onClick = onReview)
                .semantics { contentDescription = "Review approvals" }.padding(vertical = 10.dp), verticalAlignment = Alignment.CenterVertically) {
                Icon(Icons.Outlined.WarningAmber, null)
                Column(Modifier.weight(1f).padding(horizontal = 8.dp)) {
                    Text(head.tool, style = MaterialTheme.typography.labelLarge, maxLines = 1, overflow = TextOverflow.Ellipsis)
                    Text("${sideEffectLabel(head)} · $pending pending", style = MaterialTheme.typography.labelSmall, maxLines = 1,
                        overflow = TextOverflow.Ellipsis)
                }
            }
            if (control) {
                val enabled = ApprovalDecisionPolicy.decides(state, head)
                IconButton(onClick = { decide(head, "deny") }, enabled = enabled, modifier = Modifier.testTag(tag("Deny", head))) {
                    Icon(Icons.Outlined.Block, contentDescription = spoken("Deny", head))
                }
                IconButton(onClick = { decide(head, "allow") }, enabled = enabled, modifier = Modifier.testTag(tag("Allow", head))) {
                    Icon(Icons.Outlined.TaskAlt, contentDescription = spoken("Allow", head))
                }
            }
        }
    }
}

@Composable
private fun ApprovalSheetContent(state: PairingState, head: RemoteApproval, index: Int, total: Int, control: Boolean,
    confirmation: ApprovalConfirmation, decide: (RemoteApproval, String) -> Unit) {
    Column(Modifier.fillMaxWidth().navigationBarsPadding().padding(horizontal = 16.dp).padding(bottom = 16.dp).testTag("approval-sheet"),
        verticalArrangement = Arrangement.spacedBy(8.dp)) {
        Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
            Text("Awaiting approval", style = MaterialTheme.typography.titleMedium, modifier = Modifier.weight(1f).asHeading())
            Text("$index/$total", style = MaterialTheme.typography.labelLarge, modifier = Modifier.testTag("approval-position"))
        }
        Column(Modifier.fillMaxWidth().heightIn(max = 360.dp).verticalScroll(rememberScrollState()), verticalArrangement = Arrangement.spacedBy(4.dp)) {
            Text(head.tool, style = MaterialTheme.typography.bodyLarge)
            Text(sideEffectLabel(head), style = MaterialTheme.typography.labelMedium)
            Text(head.requestId, style = MaterialTheme.typography.labelSmall, modifier = Modifier.testTag("approval-request").spokenAs(null))
            head.workerSessionId?.let { Text("Worker $it", style = MaterialTheme.typography.labelSmall,
                modifier = Modifier.testTag("approval-worker-$it").spokenAs("Worktree task")) }
            head.preview?.let { preview ->
                Text(preview.path, style = MaterialTheme.typography.labelLarge)
                if (preview.withheld) Text("Preview withheld", color = MaterialTheme.colorScheme.error)
                else Text(patchText(preview.patch), style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, softWrap = false,
                    modifier = Modifier.fillMaxWidth().horizontalScroll(rememberScrollState()).testTag("approval-patch"))
                if (preview.redacted && !preview.withheld) Text("Sensitive values redacted", style = MaterialTheme.typography.labelSmall)
                if (preview.truncated) Text("Preview truncated", color = MaterialTheme.colorScheme.error)
            }
            Text(head.digest.chunked(16).joinToString("\n"), style = MaterialTheme.typography.labelSmall, fontFamily = FontFamily.Monospace,
                modifier = Modifier.spokenAs(null))
        }
        if (control) {
            if (confirmation.available) Row(Modifier.fillMaxWidth(), verticalAlignment = Alignment.CenterVertically) {
                Text("Confirm with biometrics", style = MaterialTheme.typography.bodyMedium, modifier = Modifier.weight(1f))
                Switch(confirmation.enabled, { confirmation.enabled = it }, modifier = Modifier.semantics { contentDescription = "Confirm approvals with biometrics" })
            }
            val enabled = ApprovalDecisionPolicy.decides(state, head)
            Row(Modifier.fillMaxWidth(), horizontalArrangement = Arrangement.spacedBy(12.dp, Alignment.End)) {
                OutlinedButton(onClick = { decide(head, "deny") }, enabled = enabled) { Text("Deny") }
                Button(onClick = { decide(head, "allow") }, enabled = enabled) { Text("Allow once") }
            }
        } else Text("Approval control unavailable on this device", style = MaterialTheme.typography.labelMedium)
    }
}

/** Unified-diff colouring shared by approval previews and the diff page: additions, removals and hunk headers. */
@Composable
fun patchText(patch: String): AnnotatedString {
    val dark = MaterialTheme.colorScheme.surface.luminance() < 0.5f
    val added = if (dark) Color(0xFF81C995) else Color(0xFF1B6E31)
    val removed = if (dark) Color(0xFFF28B82) else Color(0xFFB3261E)
    val hunk = MaterialTheme.colorScheme.primary
    return remember(patch, dark, hunk) {
        buildAnnotatedString {
            patch.split('\n').forEachIndexed { index, line ->
                if (index > 0) append('\n')
                val color = when {
                    line.startsWith("+++") || line.startsWith("---") -> null
                    line.startsWith("+") -> added
                    line.startsWith("-") -> removed
                    line.startsWith("@@") -> hunk
                    else -> null
                }
                if (color == null) append(line) else withStyle(SpanStyle(color = color)) { append(line) }
            }
        }
    }
}

/**
 * Optional biometric (or device credential) confirmation of approval decisions, off by default and
 * remembered on this device. Uses the platform BiometricPrompt, so no extra library is needed.
 */
class ApprovalConfirmation internal constructor(private val context: Context, initial: Boolean) {
    private val preferences = context.getSharedPreferences("zenpi-ui", Context.MODE_PRIVATE)
    private var stored by mutableStateOf(initial)
    val available: Boolean = context.getSystemService(BiometricManager::class.java)?.canAuthenticate(AUTHENTICATORS) == BiometricManager.BIOMETRIC_SUCCESS
    var enabled: Boolean
        get() = stored && available
        set(value) { stored = value; preferences.edit().putBoolean(KEY, value).apply() }

    fun confirm(title: String, onConfirmed: () -> Unit) {
        BiometricPrompt.Builder(context).setTitle(title).setAllowedAuthenticators(AUTHENTICATORS).build()
            .authenticate(CancellationSignal(), context.mainExecutor, object : BiometricPrompt.AuthenticationCallback() {
                override fun onAuthenticationSucceeded(result: BiometricPrompt.AuthenticationResult) = onConfirmed()
            })
    }

    internal companion object {
        const val KEY = "approval_biometric"
        const val AUTHENTICATORS = BiometricManager.Authenticators.BIOMETRIC_STRONG or BiometricManager.Authenticators.DEVICE_CREDENTIAL
    }
}

@Composable
private fun rememberApprovalConfirmation(): ApprovalConfirmation {
    val context = LocalContext.current
    return remember(context) {
        ApprovalConfirmation(context, context.getSharedPreferences("zenpi-ui", Context.MODE_PRIVATE).getBoolean(ApprovalConfirmation.KEY, false))
    }
}
