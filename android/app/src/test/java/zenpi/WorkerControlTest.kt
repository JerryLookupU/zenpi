package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*

class WorkerControlTest {
    private val project = "b".repeat(64)
    private val tab = "c".repeat(64)
    private val status = RemoteWorkerStatus("worker-task", project, "session", tab, "running")
    private val live = HostProjection(listOf(RemoteProject(project, "Workspace", "session", workerControl = true, workerStatuses = listOf(status))),
        project, "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "a".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs(project, "2", tab, listOf(RemoteSubtab(tab, "main", "main", 2)), false))
    private fun item() = JsonObject().apply {
        addProperty("request_id", status.requestId); addProperty("project_id", project); addProperty("session_id", "session")
        addProperty("subtab_id", tab); addProperty("state", "running")
    }
    private fun listing(item: JsonObject = item(), control: Boolean = true) = JsonObject().apply {
        addProperty("mutations_available", false); addProperty("active_project_id", project)
        add("projects", JsonArray().apply { add(JsonObject().apply {
            addProperty("id", project); addProperty("title", "Workspace"); addProperty("session_id", "session")
            addProperty("worker_control", control); add("worker_statuses", JsonArray().apply { add(item) })
        }) })
    }
    @Test fun pagedTaskListBindsDigestScopeAndRejectsMixedSnapshots() {
        fun page(offset: Int = 0, next: Int = 2, total: Int = 3, more: Boolean = true) = JsonObject().apply {
            addProperty("project_id", project); addProperty("session_id", "session"); addProperty("digest", "a".repeat(64))
            addProperty("offset", offset); addProperty("next_offset", next); addProperty("total_tasks", total); addProperty("has_more", more)
            add("tasks", JsonArray().apply { repeat(next - offset) { add(item().apply { addProperty("request_id", "task-${offset + it}") }) } })
        }
        val first = WorkerStatusPageCodec.decode(page(), live, null, 0)
        assertEquals(2, first.tasks.size)
        assertEquals("workspace_list", WorkerStatusPageCodec.request(live, first, 2)["payload"].asJsonObject["operation"].asString)
        val second = WorkerStatusPageCodec.decode(page(2, 3, 3, false), live, first, 2)
        assertFalse(second.hasMore)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") }, { it.addProperty("offset", 1) },
            { it.addProperty("next_offset", 4) }, { it.addProperty("total_tasks", 1) },
            { it.addProperty("has_more", false) }, { it.addProperty("output_text", "private") })) {
            assertTrue(runCatching { WorkerStatusPageCodec.decode(page().apply(change), live, null, 0) }.isFailure)
        }
        assertTrue(runCatching { WorkerStatusPageCodec.request(live, first, 0) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(page(2, 3, 4, false), live, first, 2) }.isFailure)
    }
    @Test fun pagedTaskListRejectsOverlapsSkippedPagesUnanchoredOffsetsAndReadsPastEnd() {
        fun page(offset: Int, next: Int, total: Int = 3, requestId: String = "task-$offset") = JsonObject().apply {
            addProperty("project_id", project); addProperty("session_id", "session"); addProperty("digest", "a".repeat(64))
            addProperty("offset", offset); addProperty("next_offset", next); addProperty("total_tasks", total); addProperty("has_more", next < total)
            add("tasks", JsonArray().apply { if (next > offset) add(item().apply { addProperty("request_id", requestId) }) })
        }
        val first = WorkerStatusPageCodec.decode(page(0, 1), live, null, 0)
        val second = WorkerStatusPageCodec.decode(page(1, 2), live, first, 1)
        val last = WorkerStatusPageCodec.decode(page(2, 3), live, second, 2)
        assertFalse(last.hasMore)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(page(1, 2, requestId = "task-0"), live, first, 1) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(page(2, 3), live, first, 2) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(page(1, 2), live, null, 1) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.decode(page(3, 3), live, last, 3) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.request(live, last, 3) }.isFailure)
    }
    @Test fun retainedTaskPagesRejectNonAdjacentDuplicateIdsAndBoundNavigationMemory() {
        fun page(offset: Int, requestId: String = "task-$offset") = WorkerStatusPage(project, "session", "a".repeat(64),
            offset, offset + 1, 12, true, listOf(status.copy(requestId = requestId)))
        var history = emptyList<WorkerStatusPage>()
        var current = page(0)
        for (offset in 1..9) {
            val next = page(offset)
            history = WorkerStatusPageCodec.remember(history, current, next)
            current = next
        }
        assertEquals((2..8).toList(), history.map { it.offset })
        assertTrue(runCatching { WorkerStatusPageCodec.remember(history, current, page(10, "task-2")) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.remember(history, current, page(11)) }.isFailure)
        assertTrue(runCatching { WorkerStatusPageCodec.remember(history.reversed(), current, page(10)) }.isFailure)
        for (changed in listOf(page(10).copy(digest = "d".repeat(64)), page(10).copy(sessionId = "other"),
            page(10).copy(projectId = "other"), page(10).copy(totalTasks = 13))) {
            assertTrue(runCatching { WorkerStatusPageCodec.remember(history, current, changed) }.isFailure)
        }
        assertEquals(emptyList<WorkerStatusPage>(), WorkerStatusPageCodec.remember(history, null, page(0)))
    }
    @Test fun previousTaskPageRequiresExactOffsetAndMatchingScope() {
        fun page(offset: Int, next: Int) = WorkerStatusPage(project, "session", "a".repeat(64), offset, next, 32, next < 32,
            (offset until next).map { status.copy(requestId = "task-$it") })
        val first = page(0, 16); val second = page(16, 32)
        assertEquals(first, WorkerStatusPageCodec.previousPage(second, listOf(first), live, 0))
        assertNull(WorkerStatusPageCodec.previousPage(second, listOf(first), live, 1))
        assertNull(WorkerStatusPageCodec.previousPage(second, emptyList(), live, 0))
        val foreign = live.copy(sessionId = "other")
        assertNull(WorkerStatusPageCodec.previousPage(second, listOf(first), foreign, 0))
    }
    @Test fun liveListingDecodesWorkerAuthorityWithoutImplyingMasterPromptControl() {
        val decoded = HostProjectionCodec.projects(listing()).single()
        assertTrue(decoded.workerControl); assertFalse(decoded.promptControl)
        assertEquals(listOf(status), decoded.workerStatuses)
        assertEquals(status, WorkerStatusCodec.decode(JsonArray().apply { add(item()) }, project, "session").single())
        val legacy = listing().apply { this["projects"].asJsonArray[0].asJsonObject.apply { remove("worker_control"); remove("worker_statuses") } }
        assertFalse(HostProjectionCodec.projects(legacy).single().workerControl)
        assertTrue(HostProjectionCodec.projects(legacy).single().workerStatuses.isEmpty())
    }
    @Test fun workerStatusRejectsUnsafeFieldsOwnersIdentityCapacityAndFailure() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("subtab_id", "bad") }, { it.remove("subtab_id") }, { it.addProperty("subtab_id", 1) },
            { it.addProperty("request_id", "bad\n") }, { it.addProperty("state", "finished") },
            { it.addProperty("failure_code", "busy") }, { it.addProperty("stream_text", "bad\u0000") },
            { it.addProperty("arguments", "private") })) {
            val invalid = item(); change(invalid)
            assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(invalid) }, project, "session") }.isFailure)
        }
        assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(item()); add(item()) }, project, "session") }.isFailure)
        assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { repeat(17) { add(item().apply { addProperty("request_id", "task-$it") }) } }, project, "session") }.isFailure)
        val failure = item().apply { addProperty("state", "failed"); addProperty("failure_code", "busy") }
        assertEquals("busy", WorkerStatusCodec.decode(JsonArray().apply { add(failure) }, project, "session").single().failureCode)
        val invalid = listing().apply { this["projects"].asJsonArray[0].asJsonObject.addProperty("worker_control", "true") }
        assertTrue(runCatching { HostProjectionCodec.projects(invalid) }.isFailure)
    }
    @Test fun workerPromptPreservesExactSelectedScopeAndCannotUseCachedAuthority() {
        val command = WorkspaceCommand.workerPrompt(live, "inspect the worktree")
        val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("text", "subtab_id", "subtab_revision"), args.keySet())
        assertEquals(tab, args["subtab_id"].asString); assertEquals("2", args["subtab_revision"].toString())
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertTrue(runCatching { WorkspaceCommand.prompt(live, "master") }.isFailure)
        for (text in listOf("", "!touch file", "bad\u0000", "x".repeat(65537))) {
            assertTrue(runCatching { WorkspaceCommand.workerPrompt(live, text) }.isFailure)
        }
        for (change in listOf<(JsonObject) -> Unit>(
            { it.remove("subtab_revision") }, { it.remove("subtab_id") }, { it.addProperty("subtab_id", "bad") },
            { it.addProperty("subtab_revision", "2") }, { it.addProperty("subtab_revision", -1) },
            { it.addProperty("subtab_revision", java.math.BigInteger("18446744073709551616")) },
            { it.addProperty("cwd", "/private") })) {
            val envelope = command.envelope; change(envelope["payload"].asJsonObject["arguments"].asJsonObject)
            assertTrue(runCatching { WorkspaceCommand.restore(envelope) }.isFailure)
        }
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.projects.single().workerControl); assertTrue(cached.projects.single().workerStatuses.isEmpty())
        assertTrue(runCatching { WorkspaceCommand.workerPrompt(cached, "never offline") }.isFailure)
        val display = AuthorityLossPolicy.displayOnly(PairingState(phase = PairingPhase.ERROR, projection = live)).projection!!
        assertFalse(display.projects.single().workerControl); assertTrue(display.projects.single().workerStatuses.isEmpty())
        val pages = WorkerStatusPage(project, "session", "a".repeat(64), 0, 1, 1, false, emptyList())
        val cleared = AuthorityLossPolicy.displayOnly(PairingState(phase = PairingPhase.ERROR, projection = live,
            workerStatusesPrevious = listOf(pages), filesPrevious = listOf(WorkspaceFiles(project, "session", tab, "2", ".", emptyList(), false, null)),
            resourcesPrevious = listOf(ResourceCatalog(project, "session", "generation", emptyList(), false, "a".repeat(64), 0, 32)),
            modelPrevious = listOf(RemoteModelStatus(project, "session", "m", null, false, "1", null, "1", "0", emptyList()))))
        assertTrue(cleared.workerStatusesPrevious.isEmpty()); assertTrue(cleared.filesPrevious.isEmpty())
        assertTrue(cleared.resourcesPrevious.isEmpty()); assertTrue(cleared.modelPrevious.isEmpty())
    }
    @Test fun workerCancelRequiresLiveTargetAndExplicitRetryKeepsOriginalEnvelope() {
        val command = WorkspaceCommand.cancelWorker(live, status.requestId)
        assertEquals(setOf("request_id"), command.envelope["payload"].asJsonObject["arguments"].asJsonObject.keySet())
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(live, "foreign") }.isFailure)
        val completed = live.copy(projects = live.projects.map { it.copy(workerStatuses = listOf(status.copy(state = "succeeded"))) })
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(completed, status.requestId) }.isFailure)
        val error = PairingState(phase = PairingPhase.ERROR, projection = live)
        assertTrue(WorkspaceDispatchPolicy.allows(error, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(error.copy(projection = completed), true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(error, false, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(error, true, false, command))
        assertFalse(WorkspaceDispatchPolicy.allows(error, true, true, WorkspaceCommand.workerPrompt(live, "new")))
        val pending = error.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(command.envelope)))
        assertFalse(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.cancelWorker(live, status.requestId)))
    }
    @Test fun connectedWorkerPromptCannotBorrowAChangedActiveTabRevisionOrCapability() {
        val command = WorkspaceCommand.workerPrompt(live, "task")
        val tabs = requireNotNull(live.subtabs)
        val state = PairingState(phase = PairingPhase.CONNECTED, projection = live)
        assertTrue(WorkspaceDispatchPolicy.allows(state, true, true, command))
        for (changed in listOf(live.copy(subtabs = null), live.copy(subtabs = tabs.copy(revision = "3")),
            live.copy(subtabs = tabs.copy(projectId = "foreign")),
            live.copy(subtabs = tabs.copy(activeId = "d".repeat(64), tabs = tabs.tabs + RemoteSubtab("d".repeat(64), "Other", "in_place", 1))),
            live.copy(projects = live.projects.map { it.copy(workerControl = false) }))) {
            assertFalse(WorkspaceDispatchPolicy.allows(state.copy(projection = changed), true, true, command))
            assertTrue(WorkspaceDispatchPolicy.allows(state.copy(projection = changed, pendingWorkspace = command), true, true, command))
        }
    }
    @Test fun unknownWorkerCancellationRequiresCurrentRuntimeEvidenceAndDropsOffline() {
        val unknown = item().apply { addProperty("state", "unknown_outcome"); addProperty("runtime_active", true) }
        val decoded = WorkerStatusCodec.decode(JsonArray().apply { add(unknown) }, project, "session").single()
        assertTrue(decoded.runtimeActive); assertTrue(decoded.canCancel)
        val active = live.copy(projects = live.projects.map { it.copy(workerStatuses = listOf(decoded)) })
        val command = WorkspaceCommand.cancelWorker(active, decoded.requestId)
        assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.ERROR, projection = active), true, true, command))
        val inactive = active.copy(projects = active.projects.map { it.copy(workerStatuses = listOf(decoded.copy(runtimeActive = false))) })
        assertFalse(inactive.projects.single().workerStatuses.single().canCancel)
        assertTrue(runCatching { WorkspaceCommand.cancelWorker(inactive, decoded.requestId) }.isFailure)
        assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.ERROR, projection = inactive), true, true, command))
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("runtime_active", false) }, { it.addProperty("runtime_active", "true") }, { it.addProperty("runtime_active", 1) },
            { it.addProperty("state", "succeeded") }, { it.addProperty("state", "failed") }, { it.addProperty("state", "cancelled") },
            { it.addProperty("stream_text", "private") }, { it.addProperty("output_text", "private"); it.addProperty("output_truncated", false) })) {
            assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(unknown.deepCopy().apply(change)) }, project, "session") }.isFailure)
        }
        val encoded = ProjectionCacheCodec.encode(active)
        assertFalse(encoded.toString().contains("runtime_active"))
        assertTrue(ProjectionCacheCodec.decode(encoded).projects.single().workerStatuses.isEmpty())
        val fenced = AuthorityLossPolicy.displayOnly(PairingState(phase = PairingPhase.ERROR, projection = active))
        assertTrue(fenced.projection!!.projects.single().workerStatuses.isEmpty())
        assertFalse(WorkspaceDispatchPolicy.allows(fenced, true, true, command))
    }
    @Test fun workerAdmissionReceiptConfirmsOnlyExactTaskAndSubtabNotCompletion() {
        val command = WorkspaceCommand.workerPrompt(live, "task")
        val result = JsonObject().apply {
            addProperty("request_id", command.envelope["id"].asString); addProperty("project_id", project)
            addProperty("session_id", "session"); addProperty("job_id", "1"); addProperty("subtab_id", tab)
        }
        fun response(value: JsonObject, state: String = "accepted") = JsonObject().apply {
            addProperty("id", command.envelope["id"].asString); addProperty("channel", "control"); addProperty("execution_state", state)
            add("payload", JsonObject().apply { addProperty("type", "response"); add("result", value) })
        }
        assertEquals(result, WorkspaceReceiptCodec.validate(command, response(result)))
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("subtab_id", "d".repeat(64)) }, { it.remove("subtab_id") },
            { it.addProperty("session_id", "worker-session") }, { it.addProperty("cwd", "private") })) {
            val invalid = result.deepCopy(); change(invalid)
            assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(invalid)) }.isFailure)
        }
        assertTrue(runCatching { WorkspaceReceiptCodec.validate(command, response(result, "succeeded")) }.isFailure)
    }
    @Test fun completedOutputHasStrictBoundedFlagsAndNeverRestoresOfflineAuthority() {
        fun completed() = item().apply {
            addProperty("state", "succeeded"); addProperty("output_text", "safe result\nAPI_KEY=<redacted>")
            addProperty("output_truncated", false)
        }
        val decoded = WorkerStatusCodec.decode(JsonArray().apply { add(completed()) }, project, "session").single()
        assertEquals("safe result\nAPI_KEY=<redacted>", decoded.outputText)
        assertFalse(decoded.outputTruncated); assertFalse(decoded.outputWithheld)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.remove("output_text") }, { it.remove("output_truncated") }, { it.addProperty("output_truncated", "false") },
            { it.addProperty("state", "running") }, { it.addProperty("output_text", "x".repeat(4097)) },
            { it.addProperty("output_text", "bad\u0000") }, { it.addProperty("output_withheld", true) },
            { it.addProperty("output_text", 1) })) {
            val invalid = completed(); change(invalid)
            assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(invalid) }, project, "session") }.isFailure)
        }
        val withheld = item().apply { addProperty("state", "succeeded"); addProperty("output_withheld", true) }
        assertTrue(WorkerStatusCodec.decode(JsonArray().apply { add(withheld) }, project, "session").single().outputWithheld)
        withheld.addProperty("output_withheld", false)
        assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(withheld) }, project, "session") }.isFailure)
        val withOutput = live.copy(projects = live.projects.map { it.copy(workerStatuses = listOf(decoded)) })
        val encoded = ProjectionCacheCodec.encode(withOutput)
        assertFalse(encoded.toString().contains("safe result"))
        assertTrue(ProjectionCacheCodec.decode(encoded).projects.single().workerStatuses.isEmpty())
    }
    @Test fun liveWorkerTextIsBoundedStateScopedAndDroppedByOfflineCache() {
        val running = item().apply { addProperty("stream_text", "partial task\nAPI_KEY=<redacted>"); addProperty("stream_truncated", true) }
        val decoded = WorkerStatusCodec.decode(JsonArray().apply { add(running) }, project, "session").single()
        assertEquals("partial task\nAPI_KEY=<redacted>", decoded.streamText); assertTrue(decoded.streamTruncated)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("state", "succeeded") }, { it.addProperty("state", "cancelled") },
            { it.addProperty("stream_text", "x".repeat(4097)) }, { it.addProperty("stream_text", "bad\u0000") },
            { it.addProperty("stream_truncated", "true") }, { it.addProperty("stream_truncated", false) },
            { it.remove("stream_text") }, { it.addProperty("output_withheld", true) })) {
            val invalid = running.deepCopy(); change(invalid)
            assertTrue(runCatching { WorkerStatusCodec.decode(JsonArray().apply { add(invalid) }, project, "session") }.isFailure)
        }
        val view = live.copy(projects = live.projects.map { it.copy(workerStatuses = listOf(decoded)) })
        val cached = ProjectionCacheCodec.encode(view)
        assertFalse(cached.toString().contains("partial task")); assertFalse(cached.toString().contains("stream_text"))
        assertTrue(ProjectionCacheCodec.decode(cached).projects.single().workerStatuses.isEmpty())
    }
}
