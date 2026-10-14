package zenpi.pairing

import androidx.lifecycle.ViewModel
import androidx.lifecycle.AndroidViewModel
import android.app.Application
import androidx.lifecycle.viewModelScope
import kotlinx.coroutines.*
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.sync.Mutex
import kotlinx.coroutines.sync.withLock
import zenpi.transport.PairingClient
import zenpi.security.PairedHostStore
import zenpi.security.PairedHost
import zenpi.security.DeviceIdentity
import zenpi.transport.HostClient
import zenpi.transport.HostProjection
import zenpi.transport.WorkspaceCommand

enum class PairingPhase { OFFLINE, CONFIRMING, CONNECTING, ACCEPTED, CONNECTED, ERROR }
/** Pairing, device registration and link lifecycle. */
data class ConnectionState(
    val phase: PairingPhase = PairingPhase.OFFLINE,
    val invite: PairingInvite? = null,
    val error: String? = null,
    val registered: Boolean = false,
    val host: PairedHost? = null,
    val pendingRevoke: String? = null,
    val scanError: String? = null,
    val resetting: Boolean = false,
    val resetRequired: Boolean = false,
    val scanInProgress: Boolean = false,
)

/** Projects, tabs, the live conversation projection (approvals, prompts, terminal) and pending workspace writes. */
data class WorkspaceState(
    val selectedProjectUnavailable: Boolean = false,
    val projection: HostProjection? = null,
    val pendingWorkspace: WorkspaceCommand? = null,
    val failureCode: String? = null,
)

/** Diff review dialog. */
data class DiffState(
    val diff: zenpi.transport.RemoteDiff? = null,
    val previous: List<zenpi.transport.RemoteDiff> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
    val open: Boolean = false,
)

/** Paged desktop terminal history. */
data class TerminalHistoryState(
    val open: Boolean = false,
    val page: zenpi.transport.TerminalPage? = null,
    val previous: List<zenpi.transport.TerminalPage> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Full message / tool-argument reader. */
data class TranscriptState(
    val turn: String? = null,
    val worker: Boolean = false,
    val historyTurn: zenpi.transport.RemoteTurn? = null,
    val toolCall: zenpi.transport.RemoteToolCall? = null,
    val page: zenpi.transport.TranscriptPage? = null,
    val previousPages: List<zenpi.transport.TranscriptPage> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Worktree task history reader. */
data class WorkerHistoryState(
    val request: String? = null,
    internal val readScope: WorkerReadScope? = null,
    val page: zenpi.transport.WorkerHistoryPage? = null,
    val previous: List<zenpi.transport.WorkerHistoryPage> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Worktree task list dialog. */
data class WorkerStatusesState(
    val open: Boolean = false,
    val page: zenpi.transport.WorkerStatusPage? = null,
    val previous: List<zenpi.transport.WorkerStatusPage> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Host file browser. */
data class FilesState(
    val open: Boolean = false,
    val files: zenpi.transport.WorkspaceFiles? = null,
    val previous: List<zenpi.transport.WorkspaceFiles> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Host resource (slash command) catalog. */
data class ResourcesState(
    val open: Boolean = false,
    val catalog: zenpi.transport.ResourceCatalog? = null,
    val previous: List<zenpi.transport.ResourceCatalog> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Host model / reasoning inspector. */
data class ModelState(
    val open: Boolean = false,
    val status: zenpi.transport.RemoteModelStatus? = null,
    val previous: List<zenpi.transport.RemoteModelStatus> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/** Session catalog / search dialog. */
data class SessionsState(
    val open: Boolean = false,
    val catalog: zenpi.transport.SessionCatalog? = null,
    val previous: List<zenpi.transport.SessionCatalog> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
    val query: String? = null,
)

/** Paged conversation history dialog. */
data class SessionHistoryState(
    val open: Boolean = false,
    val page: zenpi.transport.SessionHistoryPage? = null,
    val previous: List<zenpi.transport.SessionHistoryPage> = emptyList(),
    val loading: Boolean = false,
    val error: String? = null,
)

/**
 * Immutable screen state, composed of independent sub-states. The flat accessors,
 * the flat constructor and [copy] keep the historical single-record API so policies
 * and tests read and write fields exactly as before; equality is structural over the
 * sub-states, i.e. identical to the former data class. Flat fields are stored rather
 * than computed so callers keep smart casts on them.
 */
class PairingState(
    val connection: ConnectionState,
    val workspace: WorkspaceState,
    val diffView: DiffState,
    val terminalHistory: TerminalHistoryState,
    val transcript: TranscriptState,
    val workerHistory: WorkerHistoryState,
    val workerStatuses: WorkerStatusesState,
    val fileBrowser: FilesState,
    val resourceCatalog: ResourcesState,
    val model: ModelState,
    val sessionCatalog: SessionsState,
    val conversationHistory: SessionHistoryState,
) {
    constructor(
        phase: PairingPhase = PairingPhase.OFFLINE,
        invite: PairingInvite? = null,
        error: String? = null,
        registered: Boolean = false,
        host: PairedHost? = null,
        pendingRevoke: String? = null,
        scanError: String? = null,
        resetting: Boolean = false,
        resetRequired: Boolean = false,
        scanInProgress: Boolean = false,
        selectedProjectUnavailable: Boolean = false,
        projection: HostProjection? = null,
        pendingWorkspace: WorkspaceCommand? = null,
        workspaceFailureCode: String? = null,
        diff: zenpi.transport.RemoteDiff? = null,
        diffPrevious: List<zenpi.transport.RemoteDiff> = emptyList(),
        diffLoading: Boolean = false,
        diffError: String? = null,
        diffOpen: Boolean = false,
        terminalOpen: Boolean = false,
        terminalPage: zenpi.transport.TerminalPage? = null,
        terminalPrevious: List<zenpi.transport.TerminalPage> = emptyList(),
        terminalLoading: Boolean = false,
        terminalError: String? = null,
        transcriptTurn: String? = null,
        transcriptWorker: Boolean = false,
        transcriptHistoryTurn: zenpi.transport.RemoteTurn? = null,
        transcriptToolCall: zenpi.transport.RemoteToolCall? = null,
        transcriptPage: zenpi.transport.TranscriptPage? = null,
        transcriptPreviousPages: List<zenpi.transport.TranscriptPage> = emptyList(),
        transcriptLoading: Boolean = false,
        transcriptError: String? = null,
        workerHistoryRequest: String? = null,
        workerReadScope: WorkerReadScope? = null,
        workerHistoryPage: zenpi.transport.WorkerHistoryPage? = null,
        workerHistoryPrevious: List<zenpi.transport.WorkerHistoryPage> = emptyList(),
        workerHistoryLoading: Boolean = false,
        workerHistoryError: String? = null,
        workerStatusesOpen: Boolean = false,
        workerStatusesPage: zenpi.transport.WorkerStatusPage? = null,
        workerStatusesPrevious: List<zenpi.transport.WorkerStatusPage> = emptyList(),
        workerStatusesLoading: Boolean = false,
        workerStatusesError: String? = null,
        filesOpen: Boolean = false,
        files: zenpi.transport.WorkspaceFiles? = null,
        filesPrevious: List<zenpi.transport.WorkspaceFiles> = emptyList(),
        filesLoading: Boolean = false,
        filesError: String? = null,
        resourcesOpen: Boolean = false,
        resources: zenpi.transport.ResourceCatalog? = null,
        resourcesPrevious: List<zenpi.transport.ResourceCatalog> = emptyList(),
        resourcesLoading: Boolean = false,
        resourcesError: String? = null,
        modelOpen: Boolean = false,
        modelStatus: zenpi.transport.RemoteModelStatus? = null,
        modelPrevious: List<zenpi.transport.RemoteModelStatus> = emptyList(),
        modelLoading: Boolean = false,
        modelError: String? = null,
        sessionsOpen: Boolean = false,
        sessions: zenpi.transport.SessionCatalog? = null,
        sessionsPrevious: List<zenpi.transport.SessionCatalog> = emptyList(),
        sessionsLoading: Boolean = false,
        sessionsError: String? = null,
        sessionsQuery: String? = null,
        sessionHistoryOpen: Boolean = false,
        sessionHistory: zenpi.transport.SessionHistoryPage? = null,
        sessionHistoryPrevious: List<zenpi.transport.SessionHistoryPage> = emptyList(),
        sessionHistoryLoading: Boolean = false,
        sessionHistoryError: String? = null,
    ) : this(
        ConnectionState(phase, invite, error, registered, host, pendingRevoke, scanError, resetting, resetRequired, scanInProgress),
        WorkspaceState(selectedProjectUnavailable, projection, pendingWorkspace, workspaceFailureCode),
        DiffState(diff, diffPrevious, diffLoading, diffError, diffOpen),
        TerminalHistoryState(terminalOpen, terminalPage, terminalPrevious, terminalLoading, terminalError),
        TranscriptState(transcriptTurn, transcriptWorker, transcriptHistoryTurn, transcriptToolCall, transcriptPage, transcriptPreviousPages, transcriptLoading, transcriptError),
        WorkerHistoryState(workerHistoryRequest, workerReadScope, workerHistoryPage, workerHistoryPrevious, workerHistoryLoading, workerHistoryError),
        WorkerStatusesState(workerStatusesOpen, workerStatusesPage, workerStatusesPrevious, workerStatusesLoading, workerStatusesError),
        FilesState(filesOpen, files, filesPrevious, filesLoading, filesError),
        ResourcesState(resourcesOpen, resources, resourcesPrevious, resourcesLoading, resourcesError),
        ModelState(modelOpen, modelStatus, modelPrevious, modelLoading, modelError),
        SessionsState(sessionsOpen, sessions, sessionsPrevious, sessionsLoading, sessionsError, sessionsQuery),
        SessionHistoryState(sessionHistoryOpen, sessionHistory, sessionHistoryPrevious, sessionHistoryLoading, sessionHistoryError),
    )

    val phase: PairingPhase = connection.phase
    val invite: PairingInvite? = connection.invite
    val error: String? = connection.error
    val registered: Boolean = connection.registered
    val host: PairedHost? = connection.host
    val pendingRevoke: String? = connection.pendingRevoke
    val scanError: String? = connection.scanError
    val resetting: Boolean = connection.resetting
    val resetRequired: Boolean = connection.resetRequired
    val scanInProgress: Boolean = connection.scanInProgress
    val selectedProjectUnavailable: Boolean = workspace.selectedProjectUnavailable
    val projection: HostProjection? = workspace.projection
    val pendingWorkspace: WorkspaceCommand? = workspace.pendingWorkspace
    val workspaceFailureCode: String? = workspace.failureCode
    val diff: zenpi.transport.RemoteDiff? = diffView.diff
    val diffPrevious: List<zenpi.transport.RemoteDiff> = diffView.previous
    val diffLoading: Boolean = diffView.loading
    val diffError: String? = diffView.error
    val diffOpen: Boolean = diffView.open
    val terminalOpen: Boolean = terminalHistory.open
    val terminalPage: zenpi.transport.TerminalPage? = terminalHistory.page
    val terminalPrevious: List<zenpi.transport.TerminalPage> = terminalHistory.previous
    val terminalLoading: Boolean = terminalHistory.loading
    val terminalError: String? = terminalHistory.error
    val transcriptTurn: String? = transcript.turn
    val transcriptWorker: Boolean = transcript.worker
    val transcriptHistoryTurn: zenpi.transport.RemoteTurn? = transcript.historyTurn
    val transcriptToolCall: zenpi.transport.RemoteToolCall? = transcript.toolCall
    val transcriptPage: zenpi.transport.TranscriptPage? = transcript.page
    val transcriptPreviousPages: List<zenpi.transport.TranscriptPage> = transcript.previousPages
    val transcriptLoading: Boolean = transcript.loading
    val transcriptError: String? = transcript.error
    val workerHistoryRequest: String? = workerHistory.request
    internal val workerReadScope: WorkerReadScope? = workerHistory.readScope
    val workerHistoryPage: zenpi.transport.WorkerHistoryPage? = workerHistory.page
    val workerHistoryPrevious: List<zenpi.transport.WorkerHistoryPage> = workerHistory.previous
    val workerHistoryLoading: Boolean = workerHistory.loading
    val workerHistoryError: String? = workerHistory.error
    val workerStatusesOpen: Boolean = workerStatuses.open
    val workerStatusesPage: zenpi.transport.WorkerStatusPage? = workerStatuses.page
    val workerStatusesPrevious: List<zenpi.transport.WorkerStatusPage> = workerStatuses.previous
    val workerStatusesLoading: Boolean = workerStatuses.loading
    val workerStatusesError: String? = workerStatuses.error
    val filesOpen: Boolean = fileBrowser.open
    val files: zenpi.transport.WorkspaceFiles? = fileBrowser.files
    val filesPrevious: List<zenpi.transport.WorkspaceFiles> = fileBrowser.previous
    val filesLoading: Boolean = fileBrowser.loading
    val filesError: String? = fileBrowser.error
    val resourcesOpen: Boolean = resourceCatalog.open
    val resources: zenpi.transport.ResourceCatalog? = resourceCatalog.catalog
    val resourcesPrevious: List<zenpi.transport.ResourceCatalog> = resourceCatalog.previous
    val resourcesLoading: Boolean = resourceCatalog.loading
    val resourcesError: String? = resourceCatalog.error
    val modelOpen: Boolean = model.open
    val modelStatus: zenpi.transport.RemoteModelStatus? = model.status
    val modelPrevious: List<zenpi.transport.RemoteModelStatus> = model.previous
    val modelLoading: Boolean = model.loading
    val modelError: String? = model.error
    val sessionsOpen: Boolean = sessionCatalog.open
    val sessions: zenpi.transport.SessionCatalog? = sessionCatalog.catalog
    val sessionsPrevious: List<zenpi.transport.SessionCatalog> = sessionCatalog.previous
    val sessionsLoading: Boolean = sessionCatalog.loading
    val sessionsError: String? = sessionCatalog.error
    val sessionsQuery: String? = sessionCatalog.query
    val sessionHistoryOpen: Boolean = conversationHistory.open
    val sessionHistory: zenpi.transport.SessionHistoryPage? = conversationHistory.page
    val sessionHistoryPrevious: List<zenpi.transport.SessionHistoryPage> = conversationHistory.previous
    val sessionHistoryLoading: Boolean = conversationHistory.loading
    val sessionHistoryError: String? = conversationHistory.error

    val canScan: Boolean get() = !scanInProgress && !resetting && !resetRequired && phase != PairingPhase.CONNECTING && pendingWorkspace == null && pendingRevoke == null

    /** Field-level copy; unchanged sub-states keep their identity so Compose can skip them. */
    fun copy(
        phase: PairingPhase = this.phase,
        invite: PairingInvite? = this.invite,
        error: String? = this.error,
        registered: Boolean = this.registered,
        host: PairedHost? = this.host,
        pendingRevoke: String? = this.pendingRevoke,
        scanError: String? = this.scanError,
        resetting: Boolean = this.resetting,
        resetRequired: Boolean = this.resetRequired,
        scanInProgress: Boolean = this.scanInProgress,
        selectedProjectUnavailable: Boolean = this.selectedProjectUnavailable,
        projection: HostProjection? = this.projection,
        pendingWorkspace: WorkspaceCommand? = this.pendingWorkspace,
        workspaceFailureCode: String? = this.workspaceFailureCode,
        diff: zenpi.transport.RemoteDiff? = this.diff,
        diffPrevious: List<zenpi.transport.RemoteDiff> = this.diffPrevious,
        diffLoading: Boolean = this.diffLoading,
        diffError: String? = this.diffError,
        diffOpen: Boolean = this.diffOpen,
        terminalOpen: Boolean = this.terminalOpen,
        terminalPage: zenpi.transport.TerminalPage? = this.terminalPage,
        terminalPrevious: List<zenpi.transport.TerminalPage> = this.terminalPrevious,
        terminalLoading: Boolean = this.terminalLoading,
        terminalError: String? = this.terminalError,
        transcriptTurn: String? = this.transcriptTurn,
        transcriptWorker: Boolean = this.transcriptWorker,
        transcriptHistoryTurn: zenpi.transport.RemoteTurn? = this.transcriptHistoryTurn,
        transcriptToolCall: zenpi.transport.RemoteToolCall? = this.transcriptToolCall,
        transcriptPage: zenpi.transport.TranscriptPage? = this.transcriptPage,
        transcriptPreviousPages: List<zenpi.transport.TranscriptPage> = this.transcriptPreviousPages,
        transcriptLoading: Boolean = this.transcriptLoading,
        transcriptError: String? = this.transcriptError,
        workerHistoryRequest: String? = this.workerHistoryRequest,
        workerReadScope: WorkerReadScope? = this.workerReadScope,
        workerHistoryPage: zenpi.transport.WorkerHistoryPage? = this.workerHistoryPage,
        workerHistoryPrevious: List<zenpi.transport.WorkerHistoryPage> = this.workerHistoryPrevious,
        workerHistoryLoading: Boolean = this.workerHistoryLoading,
        workerHistoryError: String? = this.workerHistoryError,
        workerStatusesOpen: Boolean = this.workerStatusesOpen,
        workerStatusesPage: zenpi.transport.WorkerStatusPage? = this.workerStatusesPage,
        workerStatusesPrevious: List<zenpi.transport.WorkerStatusPage> = this.workerStatusesPrevious,
        workerStatusesLoading: Boolean = this.workerStatusesLoading,
        workerStatusesError: String? = this.workerStatusesError,
        filesOpen: Boolean = this.filesOpen,
        files: zenpi.transport.WorkspaceFiles? = this.files,
        filesPrevious: List<zenpi.transport.WorkspaceFiles> = this.filesPrevious,
        filesLoading: Boolean = this.filesLoading,
        filesError: String? = this.filesError,
        resourcesOpen: Boolean = this.resourcesOpen,
        resources: zenpi.transport.ResourceCatalog? = this.resources,
        resourcesPrevious: List<zenpi.transport.ResourceCatalog> = this.resourcesPrevious,
        resourcesLoading: Boolean = this.resourcesLoading,
        resourcesError: String? = this.resourcesError,
        modelOpen: Boolean = this.modelOpen,
        modelStatus: zenpi.transport.RemoteModelStatus? = this.modelStatus,
        modelPrevious: List<zenpi.transport.RemoteModelStatus> = this.modelPrevious,
        modelLoading: Boolean = this.modelLoading,
        modelError: String? = this.modelError,
        sessionsOpen: Boolean = this.sessionsOpen,
        sessions: zenpi.transport.SessionCatalog? = this.sessions,
        sessionsPrevious: List<zenpi.transport.SessionCatalog> = this.sessionsPrevious,
        sessionsLoading: Boolean = this.sessionsLoading,
        sessionsError: String? = this.sessionsError,
        sessionsQuery: String? = this.sessionsQuery,
        sessionHistoryOpen: Boolean = this.sessionHistoryOpen,
        sessionHistory: zenpi.transport.SessionHistoryPage? = this.sessionHistory,
        sessionHistoryPrevious: List<zenpi.transport.SessionHistoryPage> = this.sessionHistoryPrevious,
        sessionHistoryLoading: Boolean = this.sessionHistoryLoading,
        sessionHistoryError: String? = this.sessionHistoryError,
    ): PairingState = PairingState(
        ConnectionState(phase, invite, error, registered, host, pendingRevoke, scanError, resetting, resetRequired, scanInProgress).reuse(connection),
        WorkspaceState(selectedProjectUnavailable, projection, pendingWorkspace, workspaceFailureCode).reuse(workspace),
        DiffState(diff, diffPrevious, diffLoading, diffError, diffOpen).reuse(diffView),
        TerminalHistoryState(terminalOpen, terminalPage, terminalPrevious, terminalLoading, terminalError).reuse(terminalHistory),
        TranscriptState(transcriptTurn, transcriptWorker, transcriptHistoryTurn, transcriptToolCall, transcriptPage, transcriptPreviousPages, transcriptLoading, transcriptError).reuse(transcript),
        WorkerHistoryState(workerHistoryRequest, workerReadScope, workerHistoryPage, workerHistoryPrevious, workerHistoryLoading, workerHistoryError).reuse(workerHistory),
        WorkerStatusesState(workerStatusesOpen, workerStatusesPage, workerStatusesPrevious, workerStatusesLoading, workerStatusesError).reuse(workerStatuses),
        FilesState(filesOpen, files, filesPrevious, filesLoading, filesError).reuse(fileBrowser),
        ResourcesState(resourcesOpen, resources, resourcesPrevious, resourcesLoading, resourcesError).reuse(resourceCatalog),
        ModelState(modelOpen, modelStatus, modelPrevious, modelLoading, modelError).reuse(model),
        SessionsState(sessionsOpen, sessions, sessionsPrevious, sessionsLoading, sessionsError, sessionsQuery).reuse(sessionCatalog),
        SessionHistoryState(sessionHistoryOpen, sessionHistory, sessionHistoryPrevious, sessionHistoryLoading, sessionHistoryError).reuse(conversationHistory),
    )

    private fun parts(): List<Any> = listOf(connection, workspace, diffView, terminalHistory, transcript, workerHistory, workerStatuses, fileBrowser, resourceCatalog, model, sessionCatalog, conversationHistory)
    override fun equals(other: Any?): Boolean = this === other || other is PairingState && parts() == other.parts()
    override fun hashCode(): Int = parts().hashCode()
    override fun toString(): String = "PairingState(" + parts().joinToString() + ")"

    private companion object {
        fun <T : Any> T.reuse(previous: T): T = if (this == previous) previous else this
    }
}

class PairingViewModel @JvmOverloads constructor(application: Application, private val hostStore: PairedHostStore = PairedHostStore(application),
    private val deviceIdentity: DeviceIdentity? = null) : AndroidViewModel(application) {
    private val mutable = MutableStateFlow(PairingState())
    val state = mutable.asStateFlow()
    private val ownedJobs = OwnedJobs()
    private fun launchOwned(block: suspend CoroutineScope.() -> Unit): Job = ownedJobs.launch(viewModelScope, block)
    private var connection: Job? = null
    private var diffJob: Job? = null
    private var diffGeneration = 0
    private var terminalJob: Job? = null
    private var terminalGeneration = 0
    private var transcriptJob: Job? = null
    private var transcriptGeneration = 0
    private var workerHistoryJob: Job? = null
    private var workerHistoryGeneration = 0
    private var workerStatusesJob: Job? = null
    private var workerStatusesGeneration = 0
    private var filesJob: Job? = null
    private var filesGeneration = 0
    private var resourcesJob: Job? = null
    private var resourcesGeneration = 0
    private var modelJob: Job? = null
    private var modelGeneration = 0
    private var sessionsJob: Job? = null
    private var sessionsGeneration = 0
    private var sessionHistoryJob: Job? = null
    private var sessionHistoryGeneration = 0
    @Volatile private var generation = 0
    private val storage = Mutex()
    private var foreground = false
    private var networkAvailable = true
    private var networkRoute: Long? = null
    private var networkObserved = false
    private var metadataLoaded = false
    private var queuedInvite: PairingInvite? = null
    private var previousPairing: PairingState? = null
    private var pairingRecoveryCancelled = false
    // Cancellation invalidates the result, but its outstanding Activity still fences a new launch.
    private var scanResultAdmitted = false
    private fun storageAdmission(current: Int, context: kotlin.coroutines.CoroutineContext,
        identity: DeviceIdentity? = null, deviceId: String? = null): () -> Unit = {
        fun checkOwnership() {
            context.ensureActive()
            if (generation != current) throw CancellationException("Storage operation replaced")
        }
        if (identity != null) zenpi.transport.AuthenticationProof.current(requireNotNull(deviceId), { identity.deviceId }, ::checkOwnership)
        else checkOwnership()
    }
    private suspend fun existingIdentity(host: PairedHost): DeviceIdentity = withContext(Dispatchers.IO) {
        try {
            (deviceIdentity ?: DeviceIdentity.loadExisting() ?: throw zenpi.transport.HostAuthenticationException()).also {
                if (it.deviceId != host.deviceId) throw zenpi.transport.HostAuthenticationException()
            }
        } catch (cancelled: CancellationException) { throw cancelled }
        catch (_: Exception) { throw zenpi.transport.HostAuthenticationException() }
    }
    private fun fenceAuthority(error: Exception, stopConnection: Boolean = true): Boolean {
        if (!AuthorityLossPolicy.refused(error)) return false
        if (stopConnection) { generation++; connection?.cancel(); connection = null }
        closeDiff()
        mutable.value = AuthorityLossPolicy.displayOnly(mutable.value).copy(phase = PairingPhase.ERROR,
            selectedProjectUnavailable = error is zenpi.transport.SelectedProjectUnavailableException,
            error = "Host access or ownership changed. Refresh to authenticate again.")
        return true
    }
    private val socketRouting = zenpi.lifecycle.LanSocketRouting(application)
    init {
        launchOwned {
            try {
                val saved = storage.withLock { withContext(Dispatchers.IO) { hostStore.loadState() } }
                if (generation == 0) {
                    if (saved != null) {
                        mutable.value = PairingState(registered = true, host = saved.host, projection = saved.projection,
                            pendingWorkspace = saved.pendingWorkspace, pendingRevoke = saved.pendingRevoke,
                            error = if (saved.pendingRevoke != null) "Host revocation unconfirmed. Check the desktop before forgetting." else if (saved.pendingWorkspace != null) "Workspace result unknown. Retry the same request to resolve it." else null)
                    }
                    // Reconnect advances generation. Finish metadata admission
                    // and process the queued invitation before starting it.
                    metadataLoaded = true
                    val invite = queuedInvite
                    queuedInvite = null
                    if (invite != null) acceptInvite(invite)
                    if (!pairingRecoveryCancelled && foreground && mutable.value.host != null && mutable.value.pendingWorkspace == null
                        && mutable.value.pendingRevoke == null) refresh()
                }
            } catch (_: Exception) {
                if (generation == 0) {
                    queuedInvite = null
                    mutable.value = PairingState(PairingPhase.ERROR, error = "Saved device metadata unavailable", resetRequired = true)
                }
            }
        }
    }
    fun closeDiff() {
        closeSessionHistory()
        closeSessions()
        closeTerminal()
        closeWorkerStatuses()
        closeModel()
        closeResources()
        closeFiles()
        closeTranscript()
        diffGeneration++; diffJob?.cancel(); diffJob = null
        mutable.value = mutable.value.copy(diff = null, diffPrevious = emptyList(), diffLoading = false, diffError = null, diffOpen = false)
    }
    fun closeSessionHistory() {
        if (!mutable.value.transcriptWorker && mutable.value.transcriptHistoryTurn != null) clearTranscriptMessage()
        sessionHistoryGeneration++; sessionHistoryJob?.cancel(); sessionHistoryJob = null
        mutable.value = mutable.value.copy(sessionHistoryOpen = false, sessionHistory = null, sessionHistoryPrevious = emptyList(),
            sessionHistoryLoading = false, sessionHistoryError = null)
    }
    fun readSessionHistory(next: Boolean = false) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !SessionHistoryReadPolicy.browse(before)
            || next && !SessionHistoryReadPolicy.navigation(before)) return
        val projection = before.projection ?: return; val host = before.host ?: return
        val previous = if (next) before.sessionHistory?.takeIf { before.sessionHistoryError == null && it.hasMore
            && zenpi.transport.SessionHistoryCodec.sameScope(it, projection) } ?: return else null
        if (previous != null && !zenpi.transport.SessionHistoryCodec.validChain(before.sessionHistoryPrevious, previous)) return
        if (runCatching { zenpi.transport.SessionHistoryCodec.request(projection, previous) }.isFailure) return
        if (!next) closeDiff()
        val current = ++sessionHistoryGeneration
        mutable.value = mutable.value.copy(sessionHistoryOpen = true, sessionHistoryLoading = true, sessionHistoryError = null)
        sessionHistoryJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).sessionHistory(host, identity, projection, previous)
                if (current == sessionHistoryGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.SessionHistoryCodec.sameScope(page, mutable.value.projection)) {
                    val history = zenpi.transport.SessionHistoryCodec.remember(before.sessionHistoryPrevious, previous, page)
                    mutable.value = mutable.value.copy(sessionHistory = page, sessionHistoryPrevious = history, sessionHistoryLoading = false)
                } else if (current == sessionHistoryGeneration) closeSessionHistory()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == sessionHistoryGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(
                sessionHistoryLoading = false, sessionHistoryError = "Conversation changed or unavailable. Refresh to restart.") }
        }
    }
    fun previousSessionHistory() {
        val before = mutable.value
        if (!foreground || !networkAvailable || !SessionHistoryReadPolicy.navigation(before)) return
        val page = before.sessionHistoryPrevious.lastOrNull() ?: return
        mutable.value = before.copy(sessionHistory = page, sessionHistoryPrevious = before.sessionHistoryPrevious.dropLast(1))
    }
    fun readSessionHistoryTurn(id: String, callId: String? = null) {
        val before = mutable.value
        if (!SessionHistoryReadPolicy.navigation(before)) return
        val turn = before.sessionHistory?.turns?.singleOrNull { it.id == id } ?: return
        val call = callId?.let { value -> turn.toolCalls.singleOrNull { it.id == value } ?: return }
        loadTranscript(id, false, false, turn, call)
    }
    fun closeSessions() {
        sessionsGeneration++; sessionsJob?.cancel(); sessionsJob = null
        mutable.value = mutable.value.copy(sessionsOpen = false, sessions = null, sessionsPrevious = emptyList(), sessionsLoading = false, sessionsError = null, sessionsQuery = null)
    }
    fun searchSessions(query: String) {
        val checked = runCatching { zenpi.transport.SessionCatalogCodec.validateQuery(query.trim()) }.getOrElse { return }
        readSessions(-1, checked.takeIf { it.isNotEmpty() })
    }
    fun readSessions(offset: Int = -1, query: String? = mutable.value.sessionsQuery) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !SessionCatalogReadPolicy.browse(before)
            || offset != -1 && !SessionCatalogReadPolicy.navigation(before)) return
        val projection = before.projection ?: return; val host = before.host ?: return
        val previous = if (offset == -1) null else before.sessions?.takeIf { zenpi.transport.SessionCatalogCodec.sameScope(it, projection) } ?: return
        val index = if (offset == -1) 0 else offset
        if (runCatching { zenpi.transport.SessionCatalogCodec.request(projection, previous, index, query) }.isFailure) return
        if (previous != null && index < previous.offset && before.sessionsPrevious.lastOrNull()?.offset != index) return
        if (offset == -1) closeDiff()
        val current = ++sessionsGeneration
        mutable.value = mutable.value.copy(sessionsOpen = true, sessionsLoading = true, sessionsError = null, sessionsQuery = query)
        sessionsJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).sessionCatalog(host, identity, projection, previous, index, query)
                if (current == sessionsGeneration && foreground && mutable.value.host == host && zenpi.transport.SessionCatalogCodec.sameScope(page, mutable.value.projection)) {
                    val pages = zenpi.transport.SessionCatalogCodec.remember(before.sessionsPrevious, previous, page)
                    mutable.value = mutable.value.copy(sessions = page, sessionsPrevious = pages, sessionsLoading = false)
                } else if (current == sessionsGeneration) closeSessions()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == sessionsGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(sessions = null,
                sessionsLoading = false, sessionsError = "Session catalog changed or unavailable. Refresh to restart.") }
        }
    }
    fun resumeSession(target: String) {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.phase != PairingPhase.CONNECTED || before.sessionsLoading || before.sessionsError != null
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.SessionCommandCodec.resume(before.projection ?: return, before.sessions ?: return, target) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun closeTerminal() {
        terminalGeneration++; terminalJob?.cancel(); terminalJob = null
        mutable.value = mutable.value.copy(terminalOpen = false, terminalPage = null, terminalPrevious = emptyList(), terminalLoading = false, terminalError = null)
    }
    fun readTerminal(next: Boolean = false) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !TerminalReadPolicy.browse(before)
            || next && !TerminalReadPolicy.navigation(before)) return
        val projection = before.projection ?: return
        val host = before.host ?: return
        if (zenpi.transport.TerminalPageCodec.terminal(projection) == null) return
        val previous = if (next) before.terminalPage?.takeIf { before.terminalOpen && it.hasMore } ?: return else null
        if (previous != null && !zenpi.transport.TerminalPageCodec.validChain(before.terminalPrevious, previous)) return
        if (runCatching { zenpi.transport.TerminalPageCodec.request(projection, previous) }.isFailure) return
        if (!next) closeDiff()
        val current = ++terminalGeneration
        mutable.value = mutable.value.copy(terminalOpen = true, terminalLoading = true, terminalError = null)
        terminalJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).terminalPage(host, identity, projection, previous)
                if (current == terminalGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.TerminalPageCodec.sameScope(projection, mutable.value.projection)) {
                    val history = zenpi.transport.TerminalPageCodec.remember(before.terminalPrevious, previous, page)
                    mutable.value = mutable.value.copy(terminalPage = page, terminalLoading = false,
                        terminalPrevious = history)
                } else if (current == terminalGeneration) closeTerminal()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == terminalGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(terminalLoading = false,
                terminalError = "Terminal history changed or unavailable. Refresh to restart.") }
        }
    }
    fun previousTerminal() {
        val before = mutable.value
        if (!foreground || !networkAvailable || !TerminalReadPolicy.navigation(before)) return
        val page = before.terminalPage ?: return
        if (!zenpi.transport.TerminalPageCodec.matchesSelection(page, before.projection)) return
        val previous = runCatching { zenpi.transport.TerminalPageCodec.previous(before.terminalPrevious, page) }.getOrNull() ?: return
        mutable.value = before.copy(terminalPage = previous, terminalPrevious = before.terminalPrevious.dropLast(1))
    }
    fun closeFiles() {
        filesGeneration++; filesJob?.cancel(); filesJob = null
        mutable.value = mutable.value.copy(filesOpen = false, files = null, filesPrevious = emptyList(), filesLoading = false, filesError = null)
    }
    fun closeResources() {
        resourcesGeneration++; resourcesJob?.cancel(); resourcesJob = null
        mutable.value = mutable.value.copy(resourcesOpen = false, resources = null, resourcesPrevious = emptyList(), resourcesLoading = false, resourcesError = null)
    }
    fun closeModel() {
        modelGeneration++; modelJob?.cancel(); modelJob = null
        mutable.value = mutable.value.copy(modelOpen = false, modelStatus = null, modelPrevious = emptyList(), modelLoading = false, modelError = null)
    }
    fun inspectModel(page: Int = -1) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !ModelReadPolicy.browse(before)
            || page >= 0 && !ModelReadPolicy.navigation(before)) return
        val projection = before.projection?.takeIf { it.selectedId != null && it.sessionId != null } ?: return
        val host = before.host ?: return
        if(page < -1)return
        if (page >= 0 && before.modelError != null) return
        val previous=if(page>=0)before.modelStatus ?: return else null
        val offset=page.coerceAtLeast(0)
        if(runCatching{zenpi.transport.ModelStatusCodec.request(projection,previous,offset)}.isFailure)return
        if (page >= 0 && previous != null && offset < (previous.catalog?.offset ?: 0)) {
            val cached = before.modelPrevious.lastOrNull()?.takeIf {
                it.catalog?.nextOffset == previous.catalog?.offset && it.catalog?.offset == offset
                    && it.projectId == previous.projectId && it.sessionId == previous.sessionId
                    && it.catalog?.snapshotId == previous.catalog?.snapshotId
                    && it.selectionRevision == previous.selectionRevision
            } ?: return
            val status = zenpi.transport.ModelStatusCodec.newest(cached, before.modelStatus, projection) ?: return
            mutable.value = before.copy(modelStatus = status, modelPrevious = before.modelPrevious.dropLast(1), modelError = null)
            return
        }
        if(page == -1)closeDiff()
        val current = ++modelGeneration
        mutable.value = mutable.value.copy(modelOpen = true, modelLoading = true, modelError = null)
        modelJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val value = HostClient(socketRouting::bind).modelStatus(host, identity, projection,previous,offset)
                if (current == modelGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.WorkspaceFilesCodec.sameSelection(projection, mutable.value.projection)) {
                    val history = zenpi.transport.ModelStatusCodec.remember(before.modelPrevious, previous, value)
                    val status = zenpi.transport.ModelStatusCodec.newest(value, mutable.value.projection?.modelStatus, mutable.value.projection)
                    mutable.value = mutable.value.copy(modelStatus = status, modelPrevious = if (zenpi.transport.ModelStatusCodec.retainsPage(value, status)) history else emptyList(), modelLoading = false)
                } else if (current == modelGeneration) closeModel()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == modelGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(modelStatus=null,modelLoading = false, modelError = "Model status unavailable") }
        }
    }
    fun browseResources(page: Int = -1) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !ResourceReadPolicy.browse(before)
            || page >= 0 && !ResourceReadPolicy.navigation(before)) return
        val projection = before.projection?.takeIf { it.selectedId != null && it.sessionId != null } ?: return
        val host = before.host ?: return
        if (page < -1) return
        if (page >= 0 && before.resourcesError != null) return
        val offset = page.coerceAtLeast(0)
        val previous = if (page >= 0) before.resources ?: return else null
        if (runCatching { zenpi.transport.ResourceCatalogCodec.request(projection, previous, offset) }.isFailure) return
        if (page >= 0 && previous != null && offset < previous.offset) {
            val cached = before.resourcesPrevious.lastOrNull()?.takeIf { it.nextOffset == previous.offset && it.offset == offset
                && it.projectId == previous.projectId && it.sessionId == previous.sessionId && it.snapshotId == previous.snapshotId
                && it.generation == previous.generation }
                ?: return
            mutable.value = before.copy(resources = cached, resourcesPrevious = before.resourcesPrevious.dropLast(1), resourcesError = null)
            return
        }
        if (page == -1) closeDiff()
        val current = ++resourcesGeneration
        mutable.value = mutable.value.copy(resourcesOpen = true, resourcesLoading = true, resourcesError = null)
        resourcesJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val value = HostClient(socketRouting::bind).resources(host, identity, projection, previous, offset)
                if (current == resourcesGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.WorkspaceFilesCodec.sameSelection(projection, mutable.value.projection)) {
                    val history = zenpi.transport.ResourceCatalogCodec.remember(before.resourcesPrevious, previous, value)
                    mutable.value = mutable.value.copy(resources = value, resourcesPrevious = history, resourcesLoading = false)
                } else if (current == resourcesGeneration) closeResources()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == resourcesGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(resources = null, resourcesLoading = false, resourcesError = "Resources unavailable. Reopen the catalog.") }
        }
    }
    fun reloadResources() {
        val before = mutable.value
        if (!foreground || !networkAvailable || !ResourceReadPolicy.navigation(before)) return
        val projection = before.projection ?: return
        val catalog = before.resources ?: return
        val command = runCatching { WorkspaceCommand.reloadResources(projection, catalog) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun browseFiles(path: String = ".") {
        loadFiles(path,null,0)
    }
    fun browseFilePage(offset: Int) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !FileReadPolicy.navigation(before)) return
        val current = before.files?.takeIf { it.page != null } ?: return
        if (!zenpi.transport.WorkspaceFilesCodec.matchesSelection(current, before.projection)) return
        if (offset < current.page!!.offset) {
            val page = before.filesPrevious.lastOrNull()?.takeIf {
                it.page?.nextOffset == current.page!!.offset && it.page?.offset == offset
                    && zenpi.transport.WorkspaceFilesCodec.samePageScope(it, current)
            } ?: return
            mutable.value = before.copy(files = page, filesPrevious = before.filesPrevious.dropLast(1), filesError = null)
            return
        }
        val previous = current
        loadFiles(previous.path,previous,offset)
    }
    private fun loadFiles(path: String, previous: zenpi.transport.WorkspaceFiles?, offset: Int) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !FileReadPolicy.browse(before)) return
        val projection = before.projection?.takeIf { it.subtabs != null } ?: return
        val host = before.host ?: return
        if(runCatching{zenpi.transport.WorkspaceFilesCodec.request(projection,path,previous,offset)}.isFailure)return
        if(previous==null) {
            closeDiff()
            mutable.value = mutable.value.copy(filesPrevious = emptyList())
        }
        val current = ++filesGeneration
        mutable.value = mutable.value.copy(filesOpen = true, filesLoading = true, filesError=null)
        filesJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val value = HostClient(socketRouting::bind).files(host, identity, projection, path,previous,offset)
                if (current == filesGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.WorkspaceFilesCodec.matchesSelection(value, mutable.value.projection)) {
                    val history = zenpi.transport.WorkspaceFilesCodec.remember(before.filesPrevious, previous, value)
                    mutable.value = mutable.value.copy(files = value, filesPrevious = history, filesLoading = false)
                } else if (current == filesGeneration) closeFiles()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == filesGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(files=null,filesLoading = false, filesError = "Files unavailable") }
        }
    }
    fun closeTranscript() {
        closeWorkerHistory()
        clearTranscriptMessage()
        mutable.value = mutable.value.copy(workerReadScope = null)
    }
    private fun clearTranscriptMessage() {
        transcriptGeneration++; transcriptJob?.cancel(); transcriptJob = null
        mutable.value = mutable.value.copy(transcriptTurn = null, transcriptWorker = false, transcriptHistoryTurn = null, transcriptToolCall = null, transcriptPage = null, transcriptPreviousPages = emptyList(), transcriptLoading = false, transcriptError = null)
    }
    fun dismissTranscript() {
        transcriptGeneration++; transcriptJob?.cancel(); transcriptJob = null
        mutable.value = if (!mutable.value.transcriptWorker && mutable.value.transcriptHistoryTurn != null)
            SessionHistoryReadPolicy.dismissMessage(mutable.value) else WorkerHistoryNavigation.dismissMessage(mutable.value)
    }
    fun closeWorkerHistory() {
        workerHistoryGeneration++; workerHistoryJob?.cancel(); workerHistoryJob = null
        mutable.value = mutable.value.copy(workerHistoryRequest = null, workerHistoryPage = null, workerHistoryPrevious = emptyList(), workerHistoryLoading = false, workerHistoryError = null,
            workerReadScope = if (mutable.value.transcriptWorker) mutable.value.workerReadScope else null)
    }
    fun closeWorkerStatuses() {
        workerStatusesGeneration++; workerStatusesJob?.cancel(); workerStatusesJob = null
        mutable.value = mutable.value.copy(workerStatusesOpen = false, workerStatusesPage = null, workerStatusesPrevious = emptyList(), workerStatusesLoading = false, workerStatusesError = null)
    }
    fun readWorkerStatuses(offset: Int = -1) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !WorkerStatusReadPolicy.browse(before)
            || offset >= 0 && !WorkerStatusReadPolicy.navigation(before)) return
        val projection = before.projection ?: return; val host = before.host ?: return
        val currentPage = before.workerStatusesPage
        if (offset >= 0 && currentPage != null && offset < currentPage.offset) {
            val previousPage = zenpi.transport.WorkerStatusPageCodec.previousPage(
                currentPage, before.workerStatusesPrevious, before.projection, offset)
                ?: return
            mutable.value = before.copy(workerStatusesPage = previousPage, workerStatusesPrevious = before.workerStatusesPrevious.dropLast(1), workerStatusesError = null)
            return
        }
        val previous = if (offset >= 0) currentPage ?: return else null
        val start = offset.coerceAtLeast(0)
        if (runCatching { zenpi.transport.WorkerStatusPageCodec.request(projection, previous, start) }.isFailure) return
        if (offset < 0) {
            closeDiff()
            mutable.value = mutable.value.copy(workerStatusesPrevious = emptyList())
        }
        val current = ++workerStatusesGeneration
        mutable.value = mutable.value.copy(workerStatusesOpen = true, workerStatusesLoading = true, workerStatusesError = null)
        workerStatusesJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).workerStatuses(host, identity, projection, previous, start)
                if (current == workerStatusesGeneration && foreground && mutable.value.host == host
                    && mutable.value.projection?.workspaceOwner == projection.workspaceOwner
                    && mutable.value.projection?.workspaceRevision == projection.workspaceRevision
                    && zenpi.transport.WorkerStatusPageCodec.sameScope(page, mutable.value.projection)) {
                    val history = zenpi.transport.WorkerStatusPageCodec.remember(before.workerStatusesPrevious, previous, page)
                    mutable.value = mutable.value.copy(workerStatusesPage = page, workerStatusesPrevious = history, workerStatusesLoading = false)
                } else if (current == workerStatusesGeneration) closeWorkerStatuses()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == workerStatusesGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(workerStatusesPage = null, workerStatusesLoading = false, workerStatusesError = "Task list changed or unavailable") }
        }
    }
    fun previousWorkerHistory() {
        val before = mutable.value
        if (!foreground || !networkAvailable || !WorkerHistoryNavigation.navigation(before)) return
        mutable.value = WorkerHistoryNavigation.previous(before)
    }
    fun readWorkerHistory(requestId: String, next: Boolean = false) {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.resetting || before.resetRequired || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null || before.workerHistoryLoading || next && before.transcriptTurn != null) return
        val projection = WorkerReadPolicy.projection(before, requestId) ?: return
        val host = before.host ?: return
        if (!zenpi.transport.TranscriptPageCodec.retainsTurn(projection, projection, requestId, true)) return
        val previous = if (next) before.workerHistoryPage?.takeIf { before.workerHistoryRequest == requestId && WorkerHistoryNavigation.validScope(it, projection)
            && it.hasMore && before.workerHistoryError == null } ?: return else null
        if (next && !WorkerHistoryNavigation.navigation(before)) return
        if (runCatching { zenpi.transport.WorkerHistoryCodec.request(projection, requestId, previous) }.isFailure) return
        if (!next) { closeDiff(); closeTranscript() }
        val current = ++workerHistoryGeneration
        mutable.value = mutable.value.copy(workerHistoryRequest = requestId, workerHistoryLoading = true, workerHistoryError = null,
            workerReadScope = WorkerReadPolicy.scope(projection, requestId))
        workerHistoryJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).workerHistory(host, identity, projection, requestId, previous)
                if (current == workerHistoryGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.TranscriptPageCodec.retainsTurn(projection, WorkerReadPolicy.projection(mutable.value, requestId), requestId, true))
                    mutable.value = mutable.value.copy(workerHistoryPage = page, workerHistoryLoading = false,
                        workerHistoryPrevious = WorkerHistoryNavigation.remember(mutable.value.workerHistoryPrevious, previous, page))
                else if (current == workerHistoryGeneration) closeWorkerHistory()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == workerHistoryGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(workerHistoryLoading = false, workerHistoryError = "Task history unavailable") }
        }
    }
    fun previousTranscript() {
        val before = mutable.value
        if (!foreground || before.phase != PairingPhase.CONNECTED || before.resetting || before.resetRequired || !networkAvailable
            || before.pendingWorkspace != null || before.pendingRevoke != null || before.transcriptLoading || before.transcriptError != null) return
        val previous = TranscriptReadPolicy.previous(before) ?: return
        mutable.value = before.copy(transcriptPage = previous, transcriptPreviousPages = before.transcriptPreviousPages.dropLast(1))
    }
    private fun reconcileTranscript(next: HostProjection) {
        if (mutable.value.sessionHistoryOpen && mutable.value.projection?.journalSequence != next.journalSequence) closeSessionHistory()
        if (mutable.value.sessions?.let { !zenpi.transport.SessionCatalogCodec.sameScope(it, next) } == true) closeSessions()
        if (mutable.value.terminalOpen && mutable.value.projection?.let { zenpi.transport.TerminalPageCodec.sameScope(it, next) } != true) closeTerminal()
        mutable.value.workerStatusesPage?.let {
            if (!zenpi.transport.WorkerStatusPageCodec.sameScope(it, next)
                || mutable.value.projection?.workspaceOwner != next.workspaceOwner
                || mutable.value.projection?.workspaceRevision != next.workspaceRevision) closeWorkerStatuses()
        }
        mutable.value.workerHistoryRequest?.let { id ->
            val previous = mutable.value.projection
            val readPrevious = WorkerReadPolicy.projection(mutable.value, id, previous)
            if (readPrevious == null || !zenpi.transport.TranscriptPageCodec.retainsTurn(readPrevious, WorkerReadPolicy.projection(mutable.value, id, next), id, true)) closeWorkerHistory()
        }
        val turn = mutable.value.transcriptTurn ?: return
        val previous = mutable.value.projection
        val readPrevious = if (mutable.value.transcriptWorker) WorkerReadPolicy.projection(mutable.value, turn, previous) else SessionHistoryReadPolicy.projection(mutable.value, previous)
        val readNext = if (mutable.value.transcriptWorker) WorkerReadPolicy.projection(mutable.value, turn, next) else SessionHistoryReadPolicy.projection(mutable.value, next)
        if (readPrevious == null || !zenpi.transport.TranscriptPageCodec.retainsTurn(readPrevious, readNext, turn, mutable.value.transcriptWorker)) closeTranscript()
    }
    fun readTranscript(turnId: String, next: Boolean = false) {
        loadTranscript(turnId, next, false, historyTurn = if (next) mutable.value.transcriptHistoryTurn else null,
            toolCall = if (next) mutable.value.transcriptToolCall else null)
    }
    fun readToolArguments(turnId: String, callId: String) {
        val before = mutable.value
        if (!TranscriptReadPolicy.tool(before, turnId, callId)) return
        val turn = before.projection?.turns?.singleOrNull { it.id == turnId && it.role == "assistant" } ?: return
        val call = turn.toolCalls.singleOrNull { it.id == callId } ?: return
        loadTranscript(turnId, false, false, toolCall = call)
    }
    fun readWorkerTranscript(requestId: String, next: Boolean = false) {
        loadTranscript(requestId, next, true, if (next) mutable.value.transcriptHistoryTurn else null, if (next) mutable.value.transcriptToolCall else null)
    }
    fun readWorkerHistoryTurn(id: String) {
        val before = mutable.value
        val page = before.workerHistoryPage ?: return
        if (!WorkerHistoryNavigation.navigation(before)) return
        val turn = page.turns.singleOrNull { it.id == id } ?: return
        if (page.requestId != before.workerHistoryRequest || page.projectId != before.projection?.selectedId || page.sessionId != before.projection?.sessionId) return
        loadTranscript(page.requestId, false, true, turn)
    }
    fun readWorkerToolArguments(id: String, callId: String) {
        val before = mutable.value
        val page = before.workerHistoryPage ?: return
        if (!WorkerHistoryNavigation.navigation(before)) return
        val turn = page.turns.singleOrNull { it.id == id && it.role == "assistant" } ?: return
        val call = turn.toolCalls.singleOrNull { it.id == callId } ?: return
        loadTranscript(page.requestId, false, true, turn, call)
    }
    private fun loadTranscript(turnId: String, next: Boolean, worker: Boolean, historyTurn: zenpi.transport.RemoteTurn? = null, toolCall: zenpi.transport.RemoteToolCall? = null) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !TranscriptReadPolicy.browse(before)) return
        val projection = (if (worker) WorkerReadPolicy.projection(before, turnId) else if (historyTurn != null)
            SessionHistoryReadPolicy.projection(before.copy(transcriptHistoryTurn = historyTurn), before.projection) else before.projection) ?: return
        val host = before.host ?: return
        if (!zenpi.transport.TranscriptPageCodec.retainsTurn(projection, projection, turnId, worker) || next && before.transcriptError != null) return
        val previous = if (next) before.transcriptPage?.takeIf { before.transcriptTurn == turnId && before.transcriptWorker == worker && it.hasMore } ?: return else null
        if (next && !TranscriptReadPolicy.navigation(before)) return
        val wireHistoryTurn = if (worker) historyTurn else null
        val historyRevision = if (!worker && historyTurn != null) before.sessionHistory?.journalRevision ?: return else null
        if (runCatching { zenpi.transport.TranscriptPageCodec.request(projection, turnId, previous, worker, wireHistoryTurn, toolCall, historyRevision) }.isFailure) return
        if (!next) {
            if (historyTurn != null) clearTranscriptMessage() else { closeDiff(); closeTranscript() }
        }
        transcriptJob?.cancel(); val current = ++transcriptGeneration
        mutable.value = mutable.value.copy(transcriptTurn = turnId, transcriptWorker = worker, transcriptHistoryTurn = historyTurn, transcriptToolCall = toolCall, transcriptLoading = true, transcriptError = null,
            workerReadScope = if (worker) WorkerReadPolicy.scope(projection, turnId) else null)
        transcriptJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val page = HostClient(socketRouting::bind).transcript(host, identity, projection, turnId, previous, worker, wireHistoryTurn, toolCall, historyRevision)
                if (current == transcriptGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.TranscriptPageCodec.retainsTurn(projection,
                        if (worker) WorkerReadPolicy.projection(mutable.value, turnId) else SessionHistoryReadPolicy.projection(mutable.value, mutable.value.projection), turnId, worker))
                    mutable.value = mutable.value.copy(transcriptPage = page, transcriptLoading = false,
                        transcriptPreviousPages = zenpi.transport.TranscriptNavigation.remember(mutable.value.transcriptPreviousPages, previous, page))
                else if (current == transcriptGeneration) closeTranscript()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == transcriptGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(transcriptLoading = false,
                transcriptError = "Message page unavailable. Close and reopen to restart.") }
        }
    }
    fun reviewDiff(path: String = ".") {
        loadDiff(path, false)
    }
    fun nextDiff() { mutable.value.diff?.let { loadDiff(it.path, true) } }
    fun previousDiff() {
        val before = mutable.value
        if (!foreground || !networkAvailable || !DiffReadPolicy.navigation(before)) return
        val current = before.diff ?: return
        if (!zenpi.transport.DiffProjectionCodec.sameSelection(current, before.projection)) return
        val previous = runCatching { zenpi.transport.DiffProjectionCodec.previous(before.diffPrevious, current) }.getOrNull() ?: return
        mutable.value = before.copy(diff = previous, diffPrevious = before.diffPrevious.dropLast(1))
    }
    private fun loadDiff(path: String, continuation: Boolean) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !DiffReadPolicy.browse(before)
            || continuation && !DiffReadPolicy.navigation(before)) return
        val projection = before.projection ?: return
        val host = before.host ?: return
        val previous = if (continuation) before.diff?.takeIf { it.page?.hasMore == true && zenpi.transport.DiffProjectionCodec.sameSelection(it, projection) } ?: return else null
        if (runCatching { zenpi.transport.DiffProjectionCodec.request(projection, path, previous, paged = true) }.isFailure) return
        if (!continuation) closeDiff()
        val current = ++diffGeneration
        mutable.value = mutable.value.copy(diff = previous, diffLoading = true, diffError = null, diffOpen = true)
        diffJob = launchOwned {
            try {
                val identity = existingIdentity(host)
                val diff = HostClient(socketRouting::bind).diff(host, identity, projection, path, previous)
                if (current == diffGeneration && foreground && mutable.value.host == host
                    && zenpi.transport.DiffProjectionCodec.sameSelection(diff, mutable.value.projection))
                    mutable.value = mutable.value.copy(diff = diff, diffLoading = false,
                        diffPrevious = zenpi.transport.DiffProjectionCodec.remember(before.diffPrevious, previous, diff))
                else if (current == diffGeneration) closeDiff()
            } catch (error: CancellationException) { throw error }
            catch (error: Exception) { if (current == diffGeneration && !fenceAuthority(error)) mutable.value = mutable.value.copy(diffLoading = false, diffError = "Diff unavailable") }
        }
    }
    fun scanResult(contents: String?, missingPermission: Boolean = false, timedOut: Boolean = false) {
        if (!mutable.value.scanInProgress) return
        val admitted = scanResultAdmitted
        scanResultAdmitted = false
        mutable.value = mutable.value.copy(scanInProgress = false)
        if (!admitted || !mutable.value.canScan) return
        if (missingPermission || timedOut) {
            mutable.value = mutable.value.copy(scanError = if (missingPermission)
                "Camera permission denied. Allow camera access in Android settings to scan."
                else "Scan timed out. Try scanning again.")
        } else if (contents != null) acceptUri(contents)
        resumeAfterScan()
    }
    fun cameraUnavailable() {
        if (!mutable.value.scanInProgress) return
        val admitted = scanResultAdmitted
        scanResultAdmitted = false
        mutable.value = mutable.value.copy(scanInProgress = false)
        if (admitted && mutable.value.canScan) mutable.value = mutable.value.copy(scanError = "Camera unavailable")
        if (admitted) resumeAfterScan()
    }
    private fun resumeAfterScan() {
        if (foreground && mutable.value.host != null && mutable.value.pendingWorkspace == null && mutable.value.pendingRevoke == null) refresh()
    }
    fun beginScan(): Boolean {
        if (!metadataLoaded || !mutable.value.canScan) return false
        // Fence publications before launching the camera, not only when onStop arrives.
        generation++; connection?.cancel(); connection = null
        closeDiff()
        if (mutable.value.host != null) mutable.value = ConnectionCancelPolicy.stopped(mutable.value)
        scanResultAdmitted = true
        mutable.value = mutable.value.copy(scanError = null, scanInProgress = true)
        return true
    }
    fun acceptUri(uri: String) {
        if (mutable.value.resetting || mutable.value.resetRequired) return
        scanResultAdmitted = false
        if (metadataLoaded && !canAcceptInvite()) return
        val invite = try { PairingInvite.parse(uri) }
        catch (error: PairingException) { refuseInvite(error); return }
        // Queue a bounded, validated invitation with its original monotonic deadline.
        if (!metadataLoaded) { queuedInvite = invite; return }
        acceptInvite(invite)
    }
    private fun refuseInvite(error: PairingException) {
        mutable.value = mutable.value.copy(scanError = error.reason,
            phase = if (mutable.value.host == null && mutable.value.invite == null) PairingPhase.ERROR else mutable.value.phase,
            error = if (mutable.value.host == null && mutable.value.invite == null) error.reason else mutable.value.error)
    }
    private fun canAcceptInvite(): Boolean {
        if (mutable.value.resetting || mutable.value.resetRequired) return false
        // A deep link must not bypass the scanner's in-flight pairing fence.
        // The old host may already have registered this device or owned a save.
        if (mutable.value.phase == PairingPhase.CONNECTING && mutable.value.invite != null && mutable.value.host == null) return false
        if (mutable.value.pendingRevoke != null) return false
        if (mutable.value.pendingWorkspace != null) {
            mutable.value = mutable.value.copy(error = "Resolve or forget the pending workspace request before pairing again.")
            return false
        }
        return true
    }
    private fun acceptInvite(invite: PairingInvite) {
        if (!canAcceptInvite()) return
        try { invite.requireFresh() }
        catch (error: PairingException) { refuseInvite(error); return }
        if (mutable.value.host != null) {
            closeDiff()
            previousPairing = mutable.value.copy(phase = PairingPhase.OFFLINE, error = null, scanError = null,
                projection = mutable.value.projection?.takeIf { it.selectedId != null }
                    ?.let { runCatching { zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(it)) }.getOrNull() })
        }
        closeDiff(); connection?.cancel(); generation++
        mutable.value = PairingState(PairingPhase.CONFIRMING, invite, scanInProgress = mutable.value.scanInProgress)
    }
    fun confirm() {
        if (!foreground || !PairingConfirmationPolicy.allows(mutable.value)) return
        val invite = mutable.value.invite ?: return
        if (!networkAvailable) { mutable.value = mutable.value.copy(error = "Network unavailable"); return }
        val current = ++generation
        mutable.value = mutable.value.copy(phase = PairingPhase.CONNECTING, error = null)
        connection = launchOwned {
            try {
                val identity = PairingCredentialAdmission.load(storage, { generation == current }, { invite.requireFresh() }) {
                    deviceIdentity ?: DeviceIdentity.loadOrCreate()
                }
                val result = PairingClient(socketRouting::bind).pair(invite, "Android", identity)
                var saved: zenpi.security.SavedHostState? = null
                if (result.paired && result.deviceRegistered && generation == current) {
                    storage.withLock { withContext(Dispatchers.IO) { if (generation == current) {
                        ensureActive()
                        zenpi.transport.AuthenticationProof.deviceId(result.deviceId) { identity.deviceId }
                        saved = hostStore.save(PairedHost(invite.host, invite.port, invite.fingerprint, result.deviceId, legacyPin = invite.version == 1, fallbacks = invite.hints.drop(1)),
                            storageAdmission(current, coroutineContext, identity, result.deviceId))
                    } } }
                }
                if (generation == current) mutable.value = PairingState(
                    if (result.paired && result.deviceRegistered) PairingPhase.ACCEPTED else PairingPhase.ERROR,
                    if (result.paired && result.deviceRegistered) null else invite,
                    if (result.paired && result.deviceRegistered) null else "Device registration incomplete",
                    result.deviceRegistered,
                    host = if (result.deviceRegistered) PairedHost(invite.host, invite.port, invite.fingerprint, result.deviceId, legacyPin = invite.version == 1, fallbacks = invite.hints.drop(1)) else null,
                    projection = saved?.projection,
                )
                if (generation == current && saved != null) previousPairing = null
                if (generation == current && foreground && mutable.value.host != null) refresh()
            } catch (error: CancellationException) { throw error }
            catch (_: Exception) {
                if (generation == current) mutable.value = PairingState(PairingPhase.ERROR, invite,
                    "Pairing failed or outcome unknown. Check the desktop before scanning a new invitation.")
            }
        }
    }
    fun cancel() {
        if (mutable.value.resetting || mutable.value.resetRequired) return
        scanResultAdmitted = false
        val reconcilePairing = mutable.value.invite != null && mutable.value.host == null
        closeDiff()
        if (!metadataLoaded) { queuedInvite = null; pairingRecoveryCancelled = true; return }
        generation++; connection?.cancel(); connection = null
        val previous = previousPairing
        previousPairing = null
        mutable.value = (previous ?: ConnectionCancelPolicy.stopped(mutable.value)).copy(scanInProgress = mutable.value.scanInProgress)
        if (previous != null || reconcilePairing) {
            reconcilePairingStorage(PairingState(), resume = false)
        }
    }
    private fun reconcilePairingStorage(empty: PairingState, resume: Boolean) {
        metadataLoaded = false
        pairingRecoveryCancelled = false
        val current = generation
        // Read after any already-owned synchronous pairing commit, even when
        // cancellation prevented its main-owner state publication.
        launchOwned {
            try {
                val saved = storage.withLock { withContext(Dispatchers.IO) { hostStore.loadState() } }
                if (generation == current) {
                    previousPairing = null
                    val cancelled = pairingRecoveryCancelled
                    mutable.value = (if (saved == null) { if (cancelled) PairingState() else empty } else PairingState(
                        registered = true, host = saved.host, projection = saved.projection,
                        pendingWorkspace = saved.pendingWorkspace, pendingRevoke = saved.pendingRevoke)).copy(scanInProgress = mutable.value.scanInProgress)
                    metadataLoaded = true
                    val invite = queuedInvite
                    queuedInvite = null
                    if (invite != null) acceptInvite(invite)
                    if (resume && !cancelled && foreground && networkAvailable && mutable.value.host != null
                        && mutable.value.pendingWorkspace == null && mutable.value.pendingRevoke == null) refresh()
                }
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (_: Exception) {
                if (generation == current) {
                    queuedInvite = null
                    mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                        error = "Saved device metadata unavailable", resetRequired = true)
                }
            }
        }
    }
    private fun interruptPairing(): Boolean {
        val before = mutable.value
        if (!metadataLoaded || before.host != null || before.invite == null || before.phase != PairingPhase.CONNECTING) return false
        generation++; connection?.cancel(); connection = null
        closeDiff()
        val interrupted = before.copy(phase = PairingPhase.ERROR,
            error = "Pairing interrupted; outcome may be unknown. Check the desktop before scanning again.")
        mutable.value = interrupted
        reconcilePairingStorage(interrupted, resume = true)
        return true
    }
    fun setForeground(active: Boolean) {
        if (foreground == active) return
        foreground = active
        if (mutable.value.resetting || mutable.value.resetRequired) return
        if (!active && interruptPairing()) return
        if (!active) {
            closeDiff()
            mutable.value = AuthorityLossPolicy.displayOnly(mutable.value)
        }
        if (active && mutable.value.pendingWorkspace == null && mutable.value.pendingRevoke == null) refresh()
        else {
            if (connection != null) { generation++; connection?.cancel(); connection = null }
            if (mutable.value.host != null) mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE,
                error = if (mutable.value.pendingWorkspace != null) "Workspace result unknown. Refresh or retry the same request." else mutable.value.error)
            else if (mutable.value.phase == PairingPhase.CONNECTING) mutable.value = mutable.value.copy(
                phase = PairingPhase.ERROR, error = "Pairing interrupted; outcome may be unknown. Check the desktop before scanning again.")
        }
    }
    fun networkChanged(route: Long?) {
        if (networkObserved && networkRoute == route) return
        networkObserved = true; networkRoute = route; networkAvailable = route != null
        if (mutable.value.resetting || mutable.value.resetRequired) return
        if (interruptPairing()) return
        closeDiff()
        // Fence and close the owned operation before attaching through a replacement route.
        if (connection != null) { generation++; connection?.cancel(); connection = null }
        mutable.value = AuthorityLossPolicy.displayOnly(mutable.value)
        if (mutable.value.pendingRevoke != null) {
            mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE, error = "Host revocation unconfirmed. Check the desktop before forgetting.")
            return
        }
        if (mutable.value.host == null && mutable.value.phase == PairingPhase.CONNECTING) {
            mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                error = "Pairing interrupted; outcome may be unknown. Check the desktop before scanning again.")
        }
        if (!networkAvailable) {
            if (mutable.value.host != null) mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE,
                error = if (mutable.value.pendingWorkspace != null) "Workspace result unknown. Retry after the network returns." else "Network unavailable. Displayed data may be stale.")
            else if (mutable.value.phase == PairingPhase.CONNECTING) mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                error = "Pairing interrupted; outcome may be unknown. Check the desktop before scanning again.")
        } else if (mutable.value.pendingWorkspace != null) {
            mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE,
                error = "Workspace result unknown. Retry the same request to resolve it.")
        } else if (foreground) refresh()
    }
    fun workspace(operation: String, projectId: String, cwd: String? = null) {
        if (mutable.value.phase != PairingPhase.CONNECTED || mutable.value.pendingWorkspace != null) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.create(projection, projectId, operation, cwd) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun retryWorkspace() { mutable.value.pendingWorkspace?.let(::executeWorkspace) }
    fun prompt(text: String) {
        if (!foreground || !networkAvailable || !PromptControlPolicy.send(mutable.value)) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.prompt(projection, text) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun cancelPrompt(requestId: String) {
        if (!foreground || !networkAvailable || !PromptControlPolicy.active(mutable.value, requestId)) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.cancelPrompt(projection, requestId) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun workerPrompt(text: String) {
        if (!foreground || !networkAvailable || !PromptControlPolicy.workerSend(mutable.value)) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.workerPrompt(projection, text) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun concurrency(tabId: String, count: Int) {
        if (mutable.value.phase != PairingPhase.CONNECTED || mutable.value.pendingWorkspace != null || mutable.value.pendingRevoke != null) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.concurrency(projection, tabId, count) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun cancelWorker(requestId: String) {
        if (mutable.value.phase !in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR) || mutable.value.pendingWorkspace != null || mutable.value.pendingRevoke != null) return
        val projection = mutable.value.projection ?: return
        val target = WorkerTargetPolicy.cancellable(mutable.value, requestId) ?: return
        val command = runCatching { WorkspaceCommand.cancelWorker(projection, target) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun steer(requestId: String, text: String) {
        if (!foreground || !networkAvailable || !PromptControlPolicy.active(mutable.value, requestId)) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.steer(projection, requestId, text) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun approve(target: zenpi.transport.RemoteApproval, decision: String) {
        if (!foreground || !networkAvailable || !ApprovalDecisionPolicy.decides(mutable.value, target)) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.approve(projection, target, decision) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun subtab(operation: String, tabId: String, text: String? = null) {
        if (mutable.value.phase != PairingPhase.CONNECTED || mutable.value.pendingWorkspace != null) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.subtab(projection, operation, tabId, text) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun addDirectoryTab(name: String, cwd: String) {
        if (mutable.value.phase != PairingPhase.CONNECTED || mutable.value.pendingWorkspace != null) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.directoryTab(projection, name, cwd) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun addWorktreeTab(name: String) {
        if (mutable.value.phase != PairingPhase.CONNECTED || mutable.value.pendingWorkspace != null) return
        val projection = mutable.value.projection ?: return
        val command = runCatching { WorkspaceCommand.worktreeTab(projection, name) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun selectModel(model: String) {
        selectConfiguration { projection, status -> WorkspaceCommand.selectModel(projection, status, model) }
    }
    fun terminalInput(text: String) {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.TerminalCommandCodec.create(before.projection ?: return, text) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun openDesktopTerminal() {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.resetting || before.resetRequired || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.TerminalCommandCodec.open(before.projection ?: return) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun newSession() {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.SessionCommandCodec.newSession(before.projection ?: return) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun renameSession(name: String) {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.SessionCommandCodec.rename(before.projection ?: return, name) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun terminalResize(rows: Int, cols: Int) {
        val before = mutable.value
        if (!foreground || !networkAvailable || before.phase != PairingPhase.CONNECTED
            || before.pendingWorkspace != null || before.pendingRevoke != null) return
        val command = runCatching { zenpi.transport.TerminalCommandCodec.create(before.projection ?: return, rows = rows, cols = cols) }.getOrElse { return }
        executeWorkspace(command)
    }
    fun selectReasoning(effort: String?) {
        selectConfiguration { projection, status -> WorkspaceCommand.selectReasoning(projection, status, effort) }
    }
    private fun selectConfiguration(create: (HostProjection, zenpi.transport.RemoteModelStatus) -> WorkspaceCommand) {
        val before = mutable.value
        if (!foreground || !networkAvailable || !ModelReadPolicy.browse(before) || before.modelError != null) return
        val projection = before.projection ?: return
        val status = before.modelStatus ?: return
        val command = try { create(projection, status) }
            catch (_: Exception) { mutable.value = before.copy(modelError = "Model selection unavailable"); return }
        executeWorkspace(command)
    }
    private fun executeWorkspace(command: WorkspaceCommand) {
        if (!WorkspaceDispatchPolicy.allows(mutable.value, foreground, networkAvailable, command)) return
        closeDiff()
        val host = mutable.value.host ?: return
        if (!networkAvailable) { mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE,
            error = "Network unavailable; request was not sent."); return }
        connection?.cancel(); val current = ++generation
        mutable.value = mutable.value.copy(pendingWorkspace = command, phase = PairingPhase.CONNECTING, error = null, workspaceFailureCode = null)
        connection = launchOwned {
            try {
                storage.withLock { withContext(Dispatchers.IO) { if (generation == current)
                    hostStore.beginWorkspace(host, command, storageAdmission(current, coroutineContext)) } }
                if (generation == current) refresh(if (command.envelope["payload"].asJsonObject["operation"].asString in setOf("prompt", "steer", "cancel", "approve", "model_select", "reasoning_select", "resources_reload", "terminal_open", "terminal_input", "terminal_resize", "session_new", "session_resume", "session_rename")) mutable.value.projection?.selectedId else null, command)
            } catch (error: CancellationException) { throw error }
            catch (_: Exception) { if (generation == current) mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                error = "Workspace request could not be saved; it was not sent.") }
        }
    }
    /** ZE-515: learn a v1-saved host's key id over pinned TLS and persist it as the pin. */
    private suspend fun migratePin(host: PairedHost, current: Int): PairedHost {
        val hostId = HostClient(socketRouting::bind).hostId(host)
        val migrated = storage.withLock { withContext(Dispatchers.IO) {
            if (generation != current) throw CancellationException("Host read replaced")
            hostStore.migratePin(host, hostId, storageAdmission(current, coroutineContext))
        } }
        if (generation != current) throw CancellationException("Host read replaced")
        mutable.value = mutable.value.copy(host = migrated)
        return migrated
    }
    fun refresh(selectedId: String? = if (mutable.value.selectedProjectUnavailable) null else mutable.value.projection?.selectedId, command: WorkspaceCommand? = null) {
        if (!metadataLoaded || !foreground || mutable.value.scanInProgress || mutable.value.resetting || mutable.value.resetRequired) return
        if (command != null && mutable.value.pendingWorkspace?.envelope != command.envelope) return
        if (command != null || selectedId != mutable.value.projection?.selectedId) closeDiff()
        if (mutable.value.pendingRevoke != null) return
        val savedHost = mutable.value.host ?: return
        if (!networkAvailable) { mutable.value = mutable.value.copy(phase = PairingPhase.OFFLINE,
            error = "Network unavailable. Displayed data may be stale."); return }
        connection?.cancel(); val current = ++generation
        val previous = mutable.value
        // A read refresh must not flicker connected controls off after a confirmed write.
        // First attach and unresolved mutations still fence controls until confirmation.
        val readingConnected = command == null && previous.phase == PairingPhase.CONNECTED && previous.pendingWorkspace == null
        mutable.value = previous.copy(phase = if (readingConnected) PairingPhase.CONNECTED else PairingPhase.CONNECTING, error = null)
        connection = launchOwned {
            var host = savedHost
            var projection = previous.projection
            var readSelectedId = selectedId
            val backoff = zenpi.lifecycle.ReconnectBackoff()
            var confirmed = false
            do {
                var livePublicationStale = false
                fun requireLiveProjection(next: HostProjection) {
                    try { zenpi.state.ProjectionCommitPolicy.requireCurrent(mutable.value.projection, next) }
                    catch (error: zenpi.state.StaleProjectionException) { livePublicationStale = true; throw error }
                }
                try {
                    if (host.legacyPin) host = migratePin(host, current)
                    val identity = existingIdentity(host)
                    val next = HostClient(socketRouting::bind).inspect(host, readSelectedId, identity, previous = projection, workspaceCommand = command,
                        acknowledgePrevious = command == null && previous.pendingWorkspace == null,
                        subscribe = command == null && foreground && previous.pendingWorkspace == null,
                        onProjection = { next ->
                            ownerCallback(isCurrent = { generation == current }) {
                                requireLiveProjection(next)
                                storage.withLock { withContext(Dispatchers.IO) {
                                    if (generation != current) throw CancellationException("Subscription replaced")
                                    ensureActive()
                                    zenpi.transport.AuthenticationProof.deviceId(host.deviceId) { identity.deviceId }
                                    if (projection == null || zenpi.state.ProjectionCacheCodec.encode(requireNotNull(projection)) != zenpi.state.ProjectionCacheCodec.encode(next))
                                        hostStore.saveProjection(host, next, storageAdmission(current, coroutineContext, identity, host.deviceId))
                                } }
                                if (generation != current) throw CancellationException("Subscription replaced")
                                backoff.succeeded()
                                projection = next
                                reconcileTranscript(next)
                                if (mutable.value.projection?.selectedId != next.selectedId || mutable.value.projection?.sessionId != next.sessionId
                                    || mutable.value.projection?.subtabs?.activeId != next.subtabs?.activeId
                                    || mutable.value.projection?.subtabs?.revision != next.subtabs?.revision) closeDiff()
                                val model = if (mutable.value.modelOpen) zenpi.transport.ModelStatusCodec.newest(mutable.value.modelStatus, next.modelStatus, next) else null
                                mutable.value = mutable.value.copy(phase = PairingPhase.CONNECTED, projection = next, error = null, selectedProjectUnavailable = false,
                                    modelStatus = model, modelPrevious = if (zenpi.transport.ModelStatusCodec.retainsPage(mutable.value.modelStatus, model)) mutable.value.modelPrevious else emptyList())
                            }
                        },
                        onWorkspaceConfirmed = {
                            ownerCallback(isCurrent = { generation == current }) {
                                val target = requireNotNull(command)
                                storage.withLock { withContext(Dispatchers.IO) {
                                    if (generation != current) throw CancellationException("Workspace operation replaced")
                                    ensureActive()
                                    zenpi.transport.AuthenticationProof.deviceId(host.deviceId) { identity.deviceId }
                                    hostStore.finishWorkspace(host, target, checkActive = storageAdmission(current, coroutineContext, identity, host.deviceId))
                                } }
                                if (generation != current) throw CancellationException("Workspace operation replaced")
                                confirmed = true
                                mutable.value = mutable.value.copy(pendingWorkspace = null,
                                    workspaceFailureCode = null,
                                    projection = mutable.value.projection?.let { zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(it)) })
                            }
                        })
                    if (generation == current) requireLiveProjection(next)
                    storage.withLock { withContext(Dispatchers.IO) { if (generation == current) {
                        ensureActive()
                        zenpi.transport.AuthenticationProof.deviceId(host.deviceId) { identity.deviceId }
                        hostStore.saveProjection(host, next, storageAdmission(current, coroutineContext, identity, host.deviceId))
                    } } }
                    if (generation != current) throw CancellationException("Host read replaced")
                    projection = next
                    reconcileTranscript(next)
                    backoff.succeeded()
                    if (generation == current && (mutable.value.projection?.selectedId != next.selectedId || mutable.value.projection?.sessionId != next.sessionId
                        || mutable.value.projection?.subtabs?.activeId != next.subtabs?.activeId
                        || mutable.value.projection?.subtabs?.revision != next.subtabs?.revision)) closeDiff()
                    if (generation == current) {
                        val model = if (mutable.value.modelOpen) zenpi.transport.ModelStatusCodec.newest(mutable.value.modelStatus, next.modelStatus, next) else null
                        mutable.value = mutable.value.copy(phase = PairingPhase.CONNECTED, projection = projection, selectedProjectUnavailable = false,
                            modelStatus = model, modelPrevious = if (zenpi.transport.ModelStatusCodec.retainsPage(mutable.value.modelStatus, model)) mutable.value.modelPrevious else emptyList(),
                            error = if (command == null && previous.pendingWorkspace != null) "Workspace result unknown. Retry the same request to resolve it." else null,
                            pendingWorkspace = if (command == null) previous.pendingWorkspace else null)
                    }
                    if (command != null) { if (foreground && generation == current) refresh(next.selectedId); return@launchOwned }
                } catch (error: CancellationException) { throw error }
                catch (error: Exception) {
                    if (command == null && generation == current
                        && SessionChangePolicy.allows(mutable.value, error, foreground, networkAvailable)) {
                        val changed = error as zenpi.transport.SessionChangedException
                        closeDiff()
                        mutable.value = SessionChangePolicy.replacement(mutable.value)
                        projection = null
                        readSelectedId = changed.projectId
                        // Keep this generation and retry budget across desktop session races.
                        backoff.failed()
                        delay(backoff.delayMillis(false))
                        continue
                    }
                    val authorityLost = generation == current && fenceAuthority(error, stopConnection = false)
                    if (authorityLost) projection = mutable.value.projection
                    if (command != null) {
                        if (confirmed) {
                            if (generation == current) {
                                    mutable.value = mutable.value.copy(phase = PairingPhase.ERROR, pendingWorkspace = null, workspaceFailureCode = null,
                                        error = "Host confirmed the request. Display refresh unavailable.")
                                if (foreground && networkAvailable) refresh(selectedId)
                            }
                            break
                        }
                        val refused = error is zenpi.transport.WorkspaceRefusedException
                        if (refused) {
                            try { storage.withLock { withContext(Dispatchers.IO) { if (generation == current)
                                hostStore.finishWorkspace(host, command, checkActive = storageAdmission(current, coroutineContext)) } } }
                            catch (cancelled: CancellationException) { throw cancelled }
                            catch (_: Exception) { if (generation == current) mutable.value = (if (authorityLost) AuthorityLossPolicy.displayOnly(previous) else previous).copy(phase = PairingPhase.ERROR,
                                pendingWorkspace = command, error = "Workspace refusal received, but local resolution could not be saved."); break }
                        }
                        if (generation == current) mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                            pendingWorkspace = if (refused) null else command,
                            workspaceFailureCode = when (error) {
                                is java.net.SocketTimeoutException -> "socket_timeout"
                                is javax.net.ssl.SSLException -> "tls_failure"
                                is java.net.SocketException -> "socket_failure"
                                is java.io.EOFException -> "transport_eof"
                                is com.google.gson.JsonParseException -> "json_failure"
                                is IllegalStateException -> "state_failure"
                                is IllegalArgumentException -> "argument_failure"
                                is zenpi.transport.WorkspaceUnknownOutcomeException -> "unknown_outcome"
                                is zenpi.transport.WireException -> when (error.message) {
                                    "Host request incomplete" -> "incomplete_receipt"
                                    "Workspace receipt mismatch" -> "receipt_envelope"
                                    "Workspace owner receipt mismatch" -> "receipt_owner"
                                    "Workspace selection receipt mismatch" -> "receipt_selection"
                                    "Workspace revision receipt mismatch" -> "receipt_revision"
                                    "Subtab project receipt mismatch" -> "receipt_project"
                                    "Subtab revision receipt mismatch" -> "receipt_subtab_revision"
                                    "Host response mismatch" -> "response_identity"
                                    "Host disconnected" -> "disconnected"
                                    else -> "wire_failure"
                                }
                                else -> "transport_or_storage_failure"
                            },
                            error = if (refused) error.message else "Workspace result unknown. Refresh or retry the same request.")
                        break
                    }
                    backoff.failed()
                    if (error is zenpi.state.StaleProjectionException && !livePublicationStale) {
                        try {
                            val saved = storage.withLock { withContext(Dispatchers.IO) {
                                if (generation != current) throw CancellationException("Cache refresh replaced")
                                hostStore.loadState()
                            } }
                            if (saved?.host == host && saved.pendingWorkspace == null && saved.pendingRevoke == null
                                && generation == current) {
                                projection = saved.projection
                                mutable.value = mutable.value.copy(projection = saved.projection)
                                closeDiff()
                            }
                        } catch (cancelled: CancellationException) { throw cancelled }
                        catch (_: Exception) { /* Retain the prior display until another verified read. */ }
                    }
                    if (generation == current && !authorityLost) mutable.value = mutable.value.copy(phase = PairingPhase.ERROR,
                        error = "Host unavailable or project access refused. Displayed data may be stale.")
                    if (authorityLost) break
                }
                if (!foreground || !networkAvailable || generation != current) break
                delay(backoff.delayMillis(projection?.hasMoreEvents == true))
            } while (foreground && networkAvailable && generation == current)
        }
    }
    fun forget() {
        if (mutable.value.resetting) return
        scanResultAdmitted = false
        val priorConnection = connection
        val priorDiff = diffJob
        val priorTerminal = terminalJob
        val priorSessions = sessionsJob
        val priorSessionHistory = sessionHistoryJob
        val priorTranscript = transcriptJob
        val priorWorkerHistory = workerHistoryJob
        val priorWorkerStatuses = workerStatusesJob
        val priorFiles = filesJob
        val priorResources = resourcesJob
        val priorModel = modelJob
        val current = ++generation
        closeDiff()
        closeWorkerHistory()
        priorConnection?.cancel()
        connection = null
        queuedInvite = null
        previousPairing = null
        metadataLoaded = false
        mutable.value = PairingState(phase = PairingPhase.CONNECTING, resetting = true, resetRequired = true, scanInProgress = mutable.value.scanInProgress)
        viewModelScope.launch {
            try {
                priorConnection?.cancelAndJoin()
                priorDiff?.cancelAndJoin()
                priorTerminal?.cancelAndJoin()
                priorSessions?.cancelAndJoin()
                priorSessionHistory?.cancelAndJoin()
                priorTranscript?.cancelAndJoin()
                priorWorkerHistory?.cancelAndJoin()
                priorWorkerStatuses?.cancelAndJoin()
                priorFiles?.cancelAndJoin()
                priorResources?.cancelAndJoin()
                priorModel?.cancelAndJoin()
                ownedJobs.cancelAndJoin()
                storage.withLock { withContext(Dispatchers.IO) {
                    hostStore.beginLocalReset()
                    (deviceIdentity ?: DeviceIdentity.loadExisting())?.wipe()
                    hostStore.wipe()
                } }
                if (generation == current) {
                    metadataLoaded = true
                    mutable.value = PairingState(scanInProgress = mutable.value.scanInProgress)
                }
            } catch (error: CancellationException) { throw error }
            catch (_: Exception) {
                if (generation == current) mutable.value = PairingState(PairingPhase.ERROR,
                    error = "Local device removal failed. Retry local reset before pairing again.", resetRequired = true, scanInProgress = mutable.value.scanInProgress)
            }
        }
    }
    fun revokeDevice() {
        if (!foreground || !networkAvailable || !DeviceActionPolicy.revoke(mutable.value)) return
        closeDiff()
        val before = mutable.value
        val host = before.host ?: return
        if (before.phase == PairingPhase.CONNECTING || before.pendingWorkspace != null || before.pendingRevoke != null || !networkAvailable) return
        connection?.cancel()
        val current = ++generation
        val requestId = java.util.UUID.randomUUID().toString()
        mutable.value = before.copy(phase = PairingPhase.CONNECTING, error = null, pendingRevoke = requestId)
        connection = launchOwned {
            var persisted = false
            try {
                storage.withLock { withContext(Dispatchers.IO) {
                    if (generation == current) {
                        hostStore.beginRevoke(host, requestId, storageAdmission(current, coroutineContext)); persisted = true
                    }
                } }
                if (!persisted || generation != current) return@launchOwned
                val identity = existingIdentity(host)
                HostClient(socketRouting::bind).revokeSelf(host, identity, requestId)
                storage.withLock { withContext(Dispatchers.IO) {
                    if (generation == current) {
                        ensureActive()
                        val admission = storageAdmission(current, coroutineContext, identity, host.deviceId)
                        hostStore.finishRevoke(host, requestId, admission) { identity.wipe(host.deviceId, admission) }
                    }
                } }
                if (generation == current) mutable.value = PairingState(error = "Device revoked on host")
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) {
                if (generation == current) mutable.value = (if (AuthorityLossPolicy.refused(error)) AuthorityLossPolicy.displayOnly(before) else before).copy(phase = PairingPhase.ERROR,
                    pendingRevoke = if (persisted) requestId else null,
                    error = if (persisted) "Host revocation unconfirmed. Local credentials retained; check the desktop before forgetting."
                        else "Revocation intent could not be saved; request was not sent.")
            }
        }
    }
    fun checkRevocation() {
        if (!foreground || !networkAvailable || !DeviceActionPolicy.checkRevocation(mutable.value)) return
        closeDiff()
        val before = mutable.value
        val host = before.host ?: return
        val request = before.pendingRevoke ?: return
        if (!networkAvailable || before.phase == PairingPhase.CONNECTING) return
        connection?.cancel(); val current = ++generation
        mutable.value = before.copy(phase = PairingPhase.CONNECTING, error = null)
        connection = launchOwned {
            try {
                val identity = existingIdentity(host)
                val committed = HostClient(socketRouting::bind).checkRevoke(host, identity, request)
                if (committed) {
                    storage.withLock { withContext(Dispatchers.IO) {
                        if (generation == current) {
                            ensureActive()
                            val admission = storageAdmission(current, coroutineContext, identity, host.deviceId)
                            hostStore.finishRevoke(host, request, admission) { identity.wipe(host.deviceId, admission) }
                        }
                    } }
                    if (generation == current) mutable.value = PairingState(error = "Device revoked on host")
                } else if (generation == current) mutable.value = before.copy(phase = PairingPhase.ERROR,
                    error = "Host has no matching revocation receipt. Local credentials retained; check the desktop.")
            } catch (cancelled: CancellationException) { throw cancelled }
            catch (error: Exception) { if (generation == current) mutable.value = (if (AuthorityLossPolicy.refused(error)) AuthorityLossPolicy.displayOnly(before) else before).copy(phase = PairingPhase.ERROR,
                error = "Revocation status unavailable. Local credentials retained.") }
        }
    }
}
