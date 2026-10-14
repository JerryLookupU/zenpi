package zenpi.ui

import androidx.compose.runtime.Composable
import zenpi.pairing.PairingState
import zenpi.transport.RemoteApproval

/**
 * Lambda-per-intent entry point kept for the instrumentation suites, which observe
 * individual callbacks. It adapts the lambdas to [PairingActions] and renders [AppShell];
 * production code calls [AppShell] directly.
 */
@Composable
fun PairingScreen(state: PairingState, onScan: () -> Unit, onConfirm: (String) -> Unit, onCancel: () -> Unit, onForget: () -> Unit = {}, onRefresh: (String?) -> Unit = {},
    onWorkspace: (String, String, String?) -> Unit = { _, _, _ -> }, onRetryWorkspace: () -> Unit = {},
    onSubtab: (String, String, String?) -> Unit = { _, _, _ -> }, onAddDirectoryTab: (String, String) -> Unit = { _, _ -> }, onRevoke: () -> Unit = {}, onCheckRevoke: () -> Unit = {}, onPrompt: (String) -> Unit = {}, onCancelPrompt: (String) -> Unit = {},
    onApprove: (RemoteApproval, String) -> Unit = { _, _ -> }, onSteer: (String, String) -> Unit = { _, _ -> },
    onAddWorktreeTab: (String) -> Unit = {}, onDiff: (String) -> Unit = {}, onCloseDiff: () -> Unit = {},
    onNextDiff: () -> Unit = {}, onPreviousDiff: () -> Unit = {},
    onTranscript: (String, Boolean) -> Unit = { _, _ -> }, onCloseTranscript: () -> Unit = {},
    onWorkerTranscript: (String, Boolean) -> Unit = { _, _ -> },
    onWorkerHistory: (String, Boolean) -> Unit = { _, _ -> }, onCloseWorkerHistory: () -> Unit = {},
    onWorkerHistoryTurn: (String) -> Unit = {},
    onWorkerToolArguments: (String, String) -> Unit = { _, _ -> },
    onToolArguments: (String, String) -> Unit = { _, _ -> },
    onNewSession: () -> Unit = {},
    onRenameSession: (String) -> Unit = {},
    onSessions: (Int) -> Unit = {}, onCloseSessions: () -> Unit = {}, onResumeSession: (String) -> Unit = {},
    onSearchSessions: (String) -> Unit = {},
    onSessionHistory: (Boolean) -> Unit = {}, onPreviousSessionHistory: () -> Unit = {}, onCloseSessionHistory: () -> Unit = {},
    onSessionHistoryTurn: (String) -> Unit = {}, onSessionHistoryTool: (String, String) -> Unit = { _, _ -> },
    onTerminalInput: (String) -> Unit = {}, onTerminalResize: (Int, Int) -> Unit = { _, _ -> },
    onOpenDesktopTerminal: () -> Unit = {},
    onTerminalPage: (Boolean) -> Unit = {}, onPreviousTerminalPage: () -> Unit = {}, onCloseTerminal: () -> Unit = {},
    onPreviousWorkerHistory: () -> Unit = {},
    onWorkerStatuses: (Int) -> Unit = {}, onCloseWorkerStatuses: () -> Unit = {},
    onFiles: (String) -> Unit = {}, onCloseFiles: () -> Unit = {},
    onFilePage: (Int) -> Unit = {},
    onResources: (Int) -> Unit = {}, onCloseResources: () -> Unit = {}, onReloadResources: () -> Unit = {},
    onModel: () -> Unit = {}, onCloseModel: () -> Unit = {}, onSelectModel: (String) -> Unit = {}, onSelectReasoning: (String?) -> Unit = {}, onModelPage: (Int) -> Unit = {},
    onPreviousTranscript: () -> Unit = {}, onWorkerPrompt: (String) -> Unit = {}, onCancelWorker: (String) -> Unit = {}, onConcurrency: (String, Int) -> Unit = { _, _ -> }) {
    AppShell(state, object : PairingActions {
        override fun scan() = onScan()
        override fun confirm(code: String) = onConfirm(code)
        override fun cancel() = onCancel()
        override fun forget() = onForget()
        override fun refresh(projectId: String?) = onRefresh(projectId)
        override fun revoke() = onRevoke()
        override fun checkRevoke() = onCheckRevoke()
        override fun retryWorkspace() = onRetryWorkspace()
        override fun workspace(action: String, projectId: String, directory: String?) = onWorkspace(action, projectId, directory)
        override fun subtab(action: String, tabId: String, target: String?) = onSubtab(action, tabId, target)
        override fun addDirectoryTab(name: String, path: String) = onAddDirectoryTab(name, path)
        override fun addWorktreeTab(name: String) = onAddWorktreeTab(name)
        override fun concurrency(tabId: String, count: Int) = onConcurrency(tabId, count)
        override fun prompt(text: String) = onPrompt(text)
        override fun cancelPrompt(requestId: String) = onCancelPrompt(requestId)
        override fun steer(requestId: String, text: String) = onSteer(requestId, text)
        override fun transcript(turnId: String, next: Boolean) = onTranscript(turnId, next)
        override fun closeTranscript() = onCloseTranscript()
        override fun previousTranscript() = onPreviousTranscript()
        override fun toolArguments(turnId: String, callId: String) = onToolArguments(turnId, callId)
        override fun workerPrompt(text: String) = onWorkerPrompt(text)
        override fun cancelWorker(requestId: String) = onCancelWorker(requestId)
        override fun workerTranscript(requestId: String, next: Boolean) = onWorkerTranscript(requestId, next)
        override fun workerHistory(requestId: String, next: Boolean) = onWorkerHistory(requestId, next)
        override fun closeWorkerHistory() = onCloseWorkerHistory()
        override fun previousWorkerHistory() = onPreviousWorkerHistory()
        override fun workerHistoryTurn(turnId: String) = onWorkerHistoryTurn(turnId)
        override fun workerToolArguments(turnId: String, callId: String) = onWorkerToolArguments(turnId, callId)
        override fun workerStatuses(offset: Int) = onWorkerStatuses(offset)
        override fun closeWorkerStatuses() = onCloseWorkerStatuses()
        override fun approve(approval: RemoteApproval, decision: String) = onApprove(approval, decision)
        override fun newSession() = onNewSession()
        override fun renameSession(name: String) = onRenameSession(name)
        override fun sessions(offset: Int) = onSessions(offset)
        override fun searchSessions(query: String) = onSearchSessions(query)
        override fun closeSessions() = onCloseSessions()
        override fun resumeSession(sessionId: String) = onResumeSession(sessionId)
        override fun sessionHistory(next: Boolean) = onSessionHistory(next)
        override fun previousSessionHistory() = onPreviousSessionHistory()
        override fun closeSessionHistory() = onCloseSessionHistory()
        override fun sessionHistoryTurn(turnId: String) = onSessionHistoryTurn(turnId)
        override fun sessionHistoryTool(turnId: String, callId: String) = onSessionHistoryTool(turnId, callId)
        override fun terminalInput(bytes: String) = onTerminalInput(bytes)
        override fun terminalResize(rows: Int, cols: Int) = onTerminalResize(rows, cols)
        override fun openDesktopTerminal() = onOpenDesktopTerminal()
        override fun terminalPage(next: Boolean) = onTerminalPage(next)
        override fun previousTerminalPage() = onPreviousTerminalPage()
        override fun closeTerminal() = onCloseTerminal()
        override fun diff(path: String) = onDiff(path)
        override fun closeDiff() = onCloseDiff()
        override fun nextDiff() = onNextDiff()
        override fun previousDiff() = onPreviousDiff()
        override fun files(path: String) = onFiles(path)
        override fun closeFiles() = onCloseFiles()
        override fun filePage(offset: Int) = onFilePage(offset)
        override fun resources(offset: Int) = onResources(offset)
        override fun closeResources() = onCloseResources()
        override fun reloadResources() = onReloadResources()
        override fun model() = onModel()
        override fun closeModel() = onCloseModel()
        override fun selectModel(model: String) = onSelectModel(model)
        override fun selectReasoning(effort: String?) = onSelectReasoning(effort)
        override fun modelPage(offset: Int) = onModelPage(offset)
    })
}
