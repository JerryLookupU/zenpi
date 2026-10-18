package zenpi.pairing

import zenpi.transport.WorkspaceCommand

/** New writes use current scope/capabilities; unresolved writes retain only their exact identity. */
internal object WorkspaceDispatchPolicy {
    fun allows(state: PairingState, foreground: Boolean, networkAvailable: Boolean, command: WorkspaceCommand): Boolean {
        if (!foreground || !networkAvailable || state.scanInProgress || state.resetting || state.resetRequired
            || state.pendingRevoke != null || state.phase !in setOf(PairingPhase.CONNECTED, PairingPhase.ERROR, PairingPhase.OFFLINE)) return false
        state.pendingWorkspace?.let { return it.envelope == command.envelope }
        if (state.phase == PairingPhase.OFFLINE) return false
        val projection = state.projection ?: return false
        val envelope = command.envelope
        val operation = envelope["payload"].asJsonObject["operation"].asString
        val arguments = envelope["payload"].asJsonObject["arguments"].asJsonObject
        if (projection.workspaceOwner == null || projection.workspaceRevision == null
            || envelope["required_owner"].asString != projection.workspaceOwner
            || envelope["expected_revision"].toString() != projection.workspaceRevision) return false
        val project = projection.projects.singleOrNull { it.id == envelope["project_id"].asString } ?: return false
        if (project.sessionId != envelope["session_id"].asString) return false
        val workspaceOperation = operation in setOf("workspace_open", "workspace_select", "workspace_close")
        if (!workspaceOperation && (project.id != projection.selectedId || project.sessionId != projection.sessionId)) return false
        if (state.phase == PairingPhase.ERROR && operation !in setOf("steer", "cancel", "approve")) return false
        if (operation == "steer" && zenpi.transport.PromptStatusCodec.active(projection, arguments["request_id"].asString) == null) return false
        if (operation == "cancel" && !(zenpi.transport.PromptStatusCodec.active(projection, arguments["request_id"].asString) != null
                    || project.workerControl && WorkerTargetPolicy.cancellable(state, arguments["request_id"].asString) != null)) return false
        if (operation == "approve") {
            val target = project.pendingApprovals.singleOrNull {
                it.requestId == arguments["request_id"].asString && it.workerSessionId == arguments["worker_session_id"]?.asString
            } ?: return false
            if (target.digest != arguments["digest"].asString || !zenpi.transport.ApprovalProjectionCodec.current(projection, target)) return false
        }
        if (operation in setOf("model_select", "reasoning_select")) {
            val status = state.modelStatus ?: return false
            if (state.modelLoading || state.modelError != null
                || !zenpi.transport.ModelStatusCodec.selectionCurrent(status, projection)
                || !zenpi.transport.ModelStatusCodec.validChain(state.modelPrevious, status)
                || arguments["selection_revision"].toString() != status.selectionRevision) return false
            if (operation == "reasoning_select") {
                val selected = arguments["reasoning_effort"].takeUnless { it.isJsonNull }?.asString
                if (!zenpi.transport.ModelStatusCodec.reasoningCurrent(status, projection, selected)) return false
            }
        }
        if (operation == "resources_reload") {
            val catalog = state.resources ?: return false
            if (state.resourcesLoading || state.resourcesError != null
                || !zenpi.transport.ResourceCatalogCodec.sameScope(catalog, projection)
                || !zenpi.transport.ResourceCatalogCodec.validChain(state.resourcesPrevious, catalog)
                || arguments["generation"].toString() != catalog.generation
                || arguments["snapshot_id"].asString != catalog.snapshotId) return false
        }
        if (operation == "session_resume") {
            val catalog = state.sessions ?: return false
            if (state.sessionsLoading || state.sessionsError != null
                || !zenpi.transport.SessionCatalogCodec.resumable(catalog, projection, arguments["target_session_id"].asString)
                || !zenpi.transport.SessionCatalogCodec.validChain(state.sessionsPrevious, catalog)
                || catalog.snapshotId != arguments["snapshot_id"].asString) return false
        }
        if (arguments.has("subtab_revision")) {
            val tabs = zenpi.transport.SubtabProjectionCodec.current(projection) ?: return false
            if (tabs.projectId != project.id || tabs.revision != arguments["subtab_revision"].toString()
                || arguments.has("subtab_id") && tabs.tabs.singleOrNull { it.id == arguments["subtab_id"].asString } == null) return false
            if (operation in setOf("prompt", "terminal_open", "terminal_input", "terminal_resize") && tabs.activeId != arguments["subtab_id"].asString) return false
        }
        return when (operation) {
            "workspace_open", "workspace_select", "workspace_close" -> project.workspaceControl
            "prompt" -> if (arguments.has("subtab_id")) project.workerControl else project.promptControl
            "steer", "cancel", "approve" -> true
            "model_select" -> project.modelControl
            "reasoning_select" -> project.reasoningControl
            "resources_reload" -> project.resourcesReloadControl
            "terminal_open" -> project.terminalOpenControl
            "terminal_input", "terminal_resize" -> project.terminalControl && zenpi.transport.TerminalPageCodec.terminal(projection)?.let {
                it.running && it.terminalId == arguments["terminal_id"].asString && it.subtabId == arguments["subtab_id"].asString
                    && it.subtabRevision == arguments["subtab_revision"].toString()
            } == true
            "session_new", "session_resume", "session_rename" -> project.sessionControl
                && zenpi.transport.SessionCommandCodec.mainSelected(projection)
                && (operation != "session_rename" || projection.journalSequence == arguments["journal_revision"].toString())
            "set_concurrency" -> project.workspaceControl && projection.subtabs?.concurrencyControl == true
            "subtab_add" -> project.workspaceControl && projection.subtabs?.let {
                if (arguments["kind"].asString == "worktree") it.worktreeControl else it.directoryControl
            } == true
            "subtab_select", "subtab_rename", "subtab_move", "subtab_close" -> project.workspaceControl && projection.subtabs?.let {
                it.control && (operation != "subtab_close" || it.tabs.singleOrNull { tab -> tab.id == arguments["subtab_id"].asString }?.let { tab ->
                    if (tab.kind == "worktree") it.worktreeControl else tab.kind == "in_place" && it.directoryControl
                } == true) && (operation != "subtab_move" || arguments["before_id"].isJsonNull || it.tabs.singleOrNull { tab -> tab.id == arguments["before_id"].asString } != null)
            } == true
            else -> false
        }
    }
}
