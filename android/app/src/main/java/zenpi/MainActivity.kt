package zenpi

import android.content.Intent
import android.os.Bundle
import androidx.activity.ComponentActivity
import androidx.activity.compose.setContent
import androidx.activity.viewModels
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.runtime.remember
import androidx.compose.ui.graphics.Color
import com.journeyapps.barcodescanner.ScanContract
import com.journeyapps.barcodescanner.ScanOptions
import zenpi.pairing.PairingViewModel
import zenpi.transport.RemoteApproval
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

class MainActivity : ComponentActivity() {
    private val model: PairingViewModel by viewModels()
    private val network by lazy { zenpi.lifecycle.NetworkMonitor(this, model::networkChanged) }
    private val scanner = registerForActivityResult(ScanContract()) { result ->
        model.scanResult(result.contents,
            missingPermission = result.originalIntent?.getBooleanExtra("MISSING_CAMERA_PERMISSION", false) == true,
            timedOut = result.originalIntent?.getBooleanExtra("TIMEOUT", false) == true)
    }
    private fun takePairingIntent(incoming: Intent): String? {
        val uri = incoming.dataString
        // Keep the task's intent usable without retaining its one-use QR credential.
        setIntent(Intent(incoming).apply { setDataAndType(null, incoming.type) })
        return uri
    }
    private fun scan() {
        if (model.beginScan()) {
            if (!packageManager.hasSystemFeature(android.content.pm.PackageManager.FEATURE_CAMERA_ANY)) model.cameraUnavailable()
            else try {
                scanner.launch(ScanOptions().setCaptureActivity(PairingCaptureActivity::class.java).setDesiredBarcodeFormats(ScanOptions.QR_CODE)
                    .setBeepEnabled(false).setBarcodeImageEnabled(false).setOrientationLocked(false)
                    .setTimeout(120_000).addExtra("SHOW_MISSING_CAMERA_PERMISSION_DIALOG", false).setPrompt("Zenpi Desktop"))
            } catch (_: android.content.ActivityNotFoundException) { model.cameraUnavailable() }
        }
    }
    override fun onCreate(savedInstanceState: Bundle?) {
        val uri = takePairingIntent(intent)
        super.onCreate(savedInstanceState)
        if (savedInstanceState == null) uri?.let(model::acceptUri)
        setContent {
            MaterialTheme(colorScheme = lightColorScheme(primary = Color(0xFF166A4C), secondary = Color(0xFF934A25),
                background = Color(0xFFF5F7F8), surface = Color.White)) {
                val state by model.state.collectAsState()
                val actions = remember { ViewModelActions(model, ::scan) }
                AppShell(state, actions)
            }
        }
    }
    override fun onNewIntent(intent: Intent) {
        val uri = takePairingIntent(intent)
        super.onNewIntent(getIntent())
        uri?.let(model::acceptUri)
    }
    override fun onStart() { super.onStart(); zenpi.lifecycle.ResumeScheduler.cancel(this); network.start(); model.setForeground(true) }
    override fun onStop() { network.stop(); model.setForeground(false); zenpi.lifecycle.ResumeScheduler.enqueue(this); super.onStop() }
}

/** Routes every screen intent to [PairingViewModel]; the camera launch stays with the Activity. */
private class ViewModelActions(private val model: PairingViewModel, private val onScan: () -> Unit) : PairingActions {
    override fun scan() = onScan()
    override fun confirm() = model.confirm()
    override fun cancel() = model.cancel()
    override fun forget() = model.forget()
    override fun refresh(projectId: String?) = model.refresh(projectId)
    override fun revoke() = model.revokeDevice()
    override fun checkRevoke() = model.checkRevocation()
    override fun retryWorkspace() = model.retryWorkspace()
    override fun workspace(action: String, projectId: String, directory: String?) = model.workspace(action, projectId, directory)
    override fun subtab(action: String, tabId: String, target: String?) = model.subtab(action, tabId, target)
    override fun addDirectoryTab(name: String, path: String) = model.addDirectoryTab(name, path)
    override fun addWorktreeTab(name: String) = model.addWorktreeTab(name)
    override fun concurrency(tabId: String, count: Int) = model.concurrency(tabId, count)
    override fun prompt(text: String) = model.prompt(text)
    override fun cancelPrompt(requestId: String) = model.cancelPrompt(requestId)
    override fun steer(requestId: String, text: String) = model.steer(requestId, text)
    override fun transcript(turnId: String, next: Boolean) = model.readTranscript(turnId, next)
    override fun closeTranscript() = model.dismissTranscript()
    override fun previousTranscript() = model.previousTranscript()
    override fun toolArguments(turnId: String, callId: String) = model.readToolArguments(turnId, callId)
    override fun workerPrompt(text: String) = model.workerPrompt(text)
    override fun cancelWorker(requestId: String) = model.cancelWorker(requestId)
    override fun workerTranscript(requestId: String, next: Boolean) = model.readWorkerTranscript(requestId, next)
    override fun workerHistory(requestId: String, next: Boolean) = model.readWorkerHistory(requestId, next)
    override fun closeWorkerHistory() = model.closeWorkerHistory()
    override fun previousWorkerHistory() = model.previousWorkerHistory()
    override fun workerHistoryTurn(turnId: String) = model.readWorkerHistoryTurn(turnId)
    override fun workerToolArguments(turnId: String, callId: String) = model.readWorkerToolArguments(turnId, callId)
    override fun workerStatuses(offset: Int) = model.readWorkerStatuses(offset)
    override fun closeWorkerStatuses() = model.closeWorkerStatuses()
    override fun approve(approval: RemoteApproval, decision: String) = model.approve(approval, decision)
    override fun newSession() = model.newSession()
    override fun renameSession(name: String) = model.renameSession(name)
    override fun sessions(offset: Int) = model.readSessions(offset)
    override fun searchSessions(query: String) = model.searchSessions(query)
    override fun closeSessions() = model.closeSessions()
    override fun resumeSession(sessionId: String) = model.resumeSession(sessionId)
    override fun sessionHistory(next: Boolean) = model.readSessionHistory(next)
    override fun previousSessionHistory() = model.previousSessionHistory()
    override fun closeSessionHistory() = model.closeSessionHistory()
    override fun sessionHistoryTurn(turnId: String) = model.readSessionHistoryTurn(turnId)
    override fun sessionHistoryTool(turnId: String, callId: String) = model.readSessionHistoryTurn(turnId, callId)
    override fun terminalInput(bytes: String) = model.terminalInput(bytes)
    override fun terminalResize(rows: Int, cols: Int) = model.terminalResize(rows, cols)
    override fun openDesktopTerminal() = model.openDesktopTerminal()
    override fun terminalPage(next: Boolean) = model.readTerminal(next)
    override fun previousTerminalPage() = model.previousTerminal()
    override fun closeTerminal() = model.closeTerminal()
    override fun diff(path: String) = model.reviewDiff(path)
    override fun closeDiff() = model.closeDiff()
    override fun nextDiff() = model.nextDiff()
    override fun previousDiff() = model.previousDiff()
    override fun files(path: String) = model.browseFiles(path)
    override fun closeFiles() = model.closeFiles()
    override fun filePage(offset: Int) = model.browseFilePage(offset)
    override fun resources(offset: Int) = model.browseResources(offset)
    override fun closeResources() = model.closeResources()
    override fun reloadResources() = model.reloadResources()
    override fun model() = model.inspectModel()
    override fun closeModel() = model.closeModel()
    override fun selectModel(model: String) = this.model.selectModel(model)
    override fun selectReasoning(effort: String?) = model.selectReasoning(effort)
    override fun modelPage(offset: Int) = model.inspectModel(offset)
}
