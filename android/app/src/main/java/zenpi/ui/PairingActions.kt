package zenpi.ui

import zenpi.transport.RemoteApproval

/*
 * User intents, grouped by the screen area that raises them. Every method defaults to a
 * no-op so previews and tests implement only what they observe; the production
 * implementation forwards to PairingViewModel (see MainActivity).
 */

/** Pairing, device registration and link lifecycle. */
interface ConnectionActions {
    fun scan() {}
    fun confirm() {}
    fun cancel() {}
    fun forget() {}
    fun refresh(projectId: String?) {}
    fun revoke() {}
    fun checkRevoke() {}
    fun retryWorkspace() {}
}

/** Project tabs, worktree / directory subtabs and their concurrency. */
interface WorkspaceActions {
    fun workspace(action: String, projectId: String, directory: String?) {}
    fun subtab(action: String, tabId: String, target: String?) {}
    fun addDirectoryTab(name: String, path: String) {}
    fun addWorktreeTab(name: String) {}
    fun concurrency(tabId: String, count: Int) {}
}

/** Main conversation: messages, steering, transcript reader and worktree tasks. */
interface ConversationActions {
    fun prompt(text: String) {}
    fun cancelPrompt(requestId: String) {}
    fun steer(requestId: String, text: String) {}
    fun transcript(turnId: String, next: Boolean) {}
    fun closeTranscript() {}
    fun previousTranscript() {}
    fun toolArguments(turnId: String, callId: String) {}
    fun workerPrompt(text: String) {}
    fun cancelWorker(requestId: String) {}
    fun workerTranscript(requestId: String, next: Boolean) {}
    fun workerHistory(requestId: String, next: Boolean) {}
    fun closeWorkerHistory() {}
    fun previousWorkerHistory() {}
    fun workerHistoryTurn(turnId: String) {}
    fun workerToolArguments(turnId: String, callId: String) {}
    fun workerStatuses(offset: Int) {}
    fun closeWorkerStatuses() {}
}

/** Tool approval decisions. */
interface ApprovalActions {
    fun approve(approval: RemoteApproval, decision: String) {}
}

/** Session catalog, new / rename / resume and paged conversation history. */
interface SessionActions {
    fun newSession() {}
    fun renameSession(name: String) {}
    fun sessions(offset: Int) {}
    fun searchSessions(query: String) {}
    fun closeSessions() {}
    fun resumeSession(sessionId: String) {}
    fun sessionHistory(next: Boolean) {}
    fun previousSessionHistory() {}
    fun closeSessionHistory() {}
    fun sessionHistoryTurn(turnId: String) {}
    fun sessionHistoryTool(turnId: String, callId: String) {}
}

/** Live desktop terminal and its paged history. */
interface TerminalActions {
    fun terminalInput(bytes: String) {}
    fun terminalResize(rows: Int, cols: Int) {}
    fun openDesktopTerminal() {}
    fun terminalPage(next: Boolean) {}
    fun previousTerminalPage() {}
    fun closeTerminal() {}
}

/** Secondary read-only browsers: diff, files, resources and the host model. */
interface BrowserActions {
    fun diff(path: String) {}
    fun closeDiff() {}
    fun nextDiff() {}
    fun previousDiff() {}
    fun files(path: String) {}
    fun closeFiles() {}
    fun filePage(offset: Int) {}
    fun resources(offset: Int) {}
    fun closeResources() {}
    fun reloadResources() {}
    fun model() {}
    fun closeModel() {}
    fun selectModel(model: String) {}
    fun selectReasoning(effort: String?) {}
    fun modelPage(offset: Int) {}
}

/** Every screen intent; sections take only the group they raise. */
interface PairingActions : ConnectionActions, WorkspaceActions, ConversationActions, ApprovalActions,
    SessionActions, TerminalActions, BrowserActions
