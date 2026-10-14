package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class SubscriptionTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val initial = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "0", false, "0")
    private fun result(next: Int, done: Boolean) = JsonParser.parseString("""{"cursor":{"session_id":"session","next_sequence":$next},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"phase":"idle","journal_next_sequence":$next,"subscription_done":$done}""").asJsonObject
    private fun event(sequence: Int) = FrameCodec.decode("""{"schema_version":1,"channel":"event","session_id":"session","project_id":"project","turn_id":"turn-$sequence","sequence":$sequence,"payload":{"type":"event","name":"transcript_turn","data":{"id":"turn-$sequence","role":"assistant","content":"answer"}}}""".toByteArray())
    private fun transient() = result(0, false).apply {
        remove("phase"); remove("journal_next_sequence"); addProperty("subscription_transcript", false)
        add("subscription_view", JsonParser.parseString("""{"listing":{"mutations_available":false,"active_project_id":"project","required_owner":"${"b".repeat(64)}","workspace_revision":4,"projects":[{"id":"project","title":"Workspace","session_id":"session","workspace_control":false,"desktop_stream_text":"live preview","desktop_terminal":{"project_id":"project","session_id":"session","subtab_id":"${"a".repeat(64)}","subtab_revision":2,"text":"live terminal","truncated":false,"read_only":true}}]},"subtabs":{"project_id":"project","session_id":"session","schema_version":2,"required_owner":"${"b".repeat(64)}","workspace_revision":4,"subtab_revision":2,"active_subtab_id":"${"a".repeat(64)}","tabs":[{"id":"${"a".repeat(64)}","name":"main","kind":"main","concurrency":1}],"subtab_control":false,"directory_tab_control":false,"worktree_control":false,"concurrency_control":false}}"""))
    }
    private fun model(revision: Int = 7) = JsonParser.parseString("""{"project_id":"project","session_id":"session","model":"gpt-5.2","reasoning_effort":"high","registry_bound":true,"selection_revision":$revision,"capabilities":null,"budget":{"max_tokens":1000,"reserved_output_tokens":10},"reasoning_choices":[null,"low","high"],"catalog":{"snapshot_id":"${"d".repeat(64)}","offset":0,"next_offset":null,"entries":[{"id":"gpt-5.2","status":"active","selectable":true}]}}""").asJsonObject
    private fun withModel(value: JsonObject?) = transient().apply {
        getAsJsonObject("subscription_view").add("model_status", value ?: com.google.gson.JsonNull.INSTANCE)
    }
    @Test fun transientPageCannotCarryTranscriptAuthorityOrUseAForeignRetainedScope() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("phase", "running") }, { it.addProperty("phase", "idle") },
            { it.addProperty("journal_next_sequence", 0) }, { it.addProperty("journal_next_sequence", 1) },
            { it.add("phase", com.google.gson.JsonNull.INSTANCE) },
            { it.add("journal_next_sequence", com.google.gson.JsonNull.INSTANCE) })) {
            val reducer = SubscriptionReducer(initial, project)
            assertTrue(runCatching { reducer.accept(emptyList(), transient().apply(change)) }.exceptionOrNull() is WireException)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
            assertEquals("live preview", reducer.accept(emptyList(), transient()).projects.single().desktopStreamText)
        }
        for (retained in listOf(initial.copy(selectedId = "foreign"), initial.copy(sessionId = "foreign"))) {
            val reducer = SubscriptionReducer(retained, project)
            assertTrue(runCatching { reducer.accept(emptyList(), transient()) }.exceptionOrNull() is WireException)
            assertEquals(retained, reducer.projection); assertFalse(reducer.done)
        }
    }
    @Test fun workspaceRevisionRollbackCannotPublishNewerTabOrTransientControls() {
        val reducer = SubscriptionReducer(initial, project)
        val first = reducer.accept(emptyList(), transient())
        val stale = transient().apply {
            val view = getAsJsonObject("subscription_view")
            view.getAsJsonObject("listing").addProperty("workspace_revision", 3)
            view.getAsJsonObject("subtabs").apply {
                addProperty("workspace_revision", 3); addProperty("subtab_revision", 3)
            }
            view.getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
                .apply {
                    addProperty("desktop_stream_text", "stale authority update")
                    getAsJsonObject("desktop_terminal").addProperty("subtab_revision", 3)
                }
        }
        assertTrue(runCatching { reducer.accept(emptyList(), stale) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
        assertEquals(first, reducer.projection); assertFalse(reducer.done)
        val newer = transient().apply {
            val view = getAsJsonObject("subscription_view")
            view.getAsJsonObject("listing").addProperty("workspace_revision", 5)
            view.getAsJsonObject("subtabs").addProperty("workspace_revision", 5)
            addProperty("subscription_done", true)
        }
        assertEquals("5", reducer.accept(emptyList(), newer).workspaceRevision)
        assertTrue(reducer.done)
    }
    @Test fun mismatchedTerminalSelectionCannotCommitSubscriptionViewOrCursor() {
        val reducer = SubscriptionReducer(initial, project)
        val first = reducer.accept(emptyList(), transient())
        for ((field, mismatch) in listOf("subtab_id" to "c".repeat(64), "subtab_revision" to "3")) {
            val changed = transient().apply {
                val terminal = getAsJsonObject("subscription_view").getAsJsonObject("listing")
                    .getAsJsonArray("projects")[0].asJsonObject.getAsJsonObject("desktop_terminal")
                if (field == "subtab_revision") terminal.addProperty(field, mismatch.toInt()) else terminal.addProperty(field, mismatch)
                addProperty("subscription_done", true)
            }
            assertTrue(runCatching { reducer.accept(emptyList(), changed) }.exceptionOrNull() is WireException)
            assertEquals(first, reducer.projection); assertFalse(reducer.done)
        }
        val valid = transient().apply { addProperty("subscription_done", true) }
        assertEquals(first, reducer.accept(emptyList(), valid)); assertTrue(reducer.done)
    }
    @Test fun subscriptionPublicationCannotOverwriteNewerWorkspaceAfterSelectionChanges() = kotlinx.coroutines.runBlocking {
        val reducer = SubscriptionReducer(initial, project)
        val fresh = initial.copy(selectedId = "other", sessionId = "other-session", workspaceOwner = "b".repeat(64), workspaceRevision = "5")
        var visible = fresh
        var durable = fresh
        val request = JsonParser.parseString("""{"id":"sub","project_id":"project","session_id":"session","payload":{"type":"subscribe"}}""").asJsonObject
        val response = JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", "sub")
            addProperty("project_id", "project"); addProperty("session_id", "session"); addProperty("execution_state", "succeeded")
            add("payload", JsonObject().apply { addProperty("type", "response"); add("result", transient().apply { addProperty("subscription_done", true) }) })
        }
        val bytes = java.io.ByteArrayOutputStream().also { FrameCodec.write(it, response) }.toByteArray()
        val error = runCatching {
            ResponseReader.read(java.io.ByteArrayInputStream(bytes), request, {}) { batch ->
                val next = reducer.accept(batch.frames, batch.result)
                zenpi.state.ProjectionCommitPolicy.requireCurrent(durable, next)
                durable = next; visible = next
            }
        }.exceptionOrNull()
        assertTrue(error is zenpi.state.StaleProjectionException)
        assertEquals(fresh, visible); assertEquals(fresh, durable)
    }
    @Test fun localSelectionPushIsTransientMonotonicAndDoesNotAdvanceOrPersistCursor() {
        val reducer = SubscriptionReducer(initial, project)
        val first = reducer.accept(emptyList(), withModel(model()))
        assertEquals("gpt-5.2", first.modelStatus?.model)
        assertEquals("high", first.modelStatus?.reasoningEffort)
        val update = model(9).apply { addProperty("model", "gpt-5.2-2025-12-11"); addProperty("reasoning_effort", "low") }
        val next = reducer.accept(emptyList(), withModel(update))
        assertEquals("9", next.modelStatus?.selectionRevision)
        assertEquals("low", next.modelStatus?.reasoningEffort)
        assertEquals("0", next.eventSequence); assertEquals("0", next.journalSequence)
        assertEquals(initial.turns, next.turns)
        assertNull(zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next)).modelStatus)
        assertTrue(runCatching { reducer.accept(emptyList(), withModel(model(8))) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
        assertEquals(next, reducer.projection)
        assertNull(reducer.accept(emptyList(), withModel(null)).modelStatus)
        assertTrue(runCatching { reducer.accept(emptyList(), withModel(model(8))) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
    }
    @Test fun modelScopeShapeAndPrivacyFailuresDoNotCommitAnyPartOfSubscription() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("selection_revision", "7") }, { it.addProperty("endpoint", "https://private.invalid") },
            { it.getAsJsonObject("catalog").addProperty("snapshot_id", "invalid") },
            { it.getAsJsonObject("catalog").addProperty("offset", 16) },
            { it.getAsJsonObject("catalog").getAsJsonArray("entries")[0].asJsonObject.addProperty("id", "/private/model") })) {
            val reducer = SubscriptionReducer(initial, project)
            assertTrue(runCatching { reducer.accept(emptyList(), withModel(model().apply(change))) }.isFailure)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
    }
    @Test fun racingStatusReadsCannotReplaceNewerSelectionOrResetAnUnchangedCatalogPage() {
        val old = ModelStatusCodec.decode(model(), initial)
        val page = old.copy(catalog = old.catalog?.copy(offset = 16))
        assertEquals(page, ModelStatusCodec.newest(page, old, initial))
        val newer = ModelStatusCodec.decode(model(9), initial)
        assertEquals(newer, ModelStatusCodec.newest(page, newer, initial))
        assertEquals(newer, ModelStatusCodec.newest(newer, old, initial))
        val reloaded = old.copy(catalog = old.catalog?.copy(snapshotId = "e".repeat(64)))
        assertEquals(reloaded, ModelStatusCodec.newest(page, reloaded, initial))
        assertEquals(page, ModelStatusCodec.newest(page, null, initial))
        assertNull(ModelStatusCodec.newest(page, old, initial.copy(sessionId = "foreign")))
    }
    @Test fun contiguousPagesKeepCursorAndCompletionAndNeverInventAuthority() {
        val reducer = SubscriptionReducer(initial, project)
        assertEquals(initial, reducer.accept(emptyList(), result(0, false)))
        assertEquals("1", reducer.accept(listOf(event(0)), result(1, false)).eventSequence)
        assertEquals("2", reducer.accept(listOf(event(1)), result(2, true)).eventSequence)
        assertTrue(reducer.done); assertNull(reducer.projection.workspaceOwner)
        assertTrue(runCatching { reducer.accept(emptyList(), result(2, true)) }.isFailure)
    }
    @Test fun contradictoryGapSnapshotCannotCommitSubscriptionOrItsTransientView() {
        val reducer = SubscriptionReducer(initial, project)
        val snapshot = FrameCodec.decode("""{"schema_version":1,"channel":"bulk","session_id":"session","project_id":"project","payload":{"type":"snapshot","cursor":{"session_id":"session","next_sequence":2},"state":{"project_id":"project","session_id":"session","cursor_scope":"session_journal","phase":"idle","journal_next_sequence":9,"transcript_truncated":false,"mutations_available":false,"event_cursor_scope":"transcript_events_v1","event_cursor":{"session_id":"session","next_sequence":2},"transcript":[{"id":"replacement","role":"assistant","content":"fresh"}]}}}""".toByteArray())
        val terminal = transient().apply {
            getAsJsonObject("cursor").addProperty("next_sequence", 2)
            addProperty("snapshot_required", true); addProperty("subscription_transcript", true)
            addProperty("subscription_done", true); addProperty("journal_next_sequence", 9); addProperty("phase", "idle")
        }
        for (changed in listOf(terminal.deepCopy().apply { addProperty("journal_next_sequence", 8) },
            terminal.deepCopy().apply { addProperty("phase", "running") })) {
            assertTrue(runCatching { reducer.accept(listOf(snapshot), changed) }.isFailure)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
        val next = reducer.accept(listOf(snapshot), terminal)
        assertTrue(reducer.done); assertEquals("fresh", next.turns.single().content)
        assertEquals("2", next.eventSequence); assertEquals("9", next.journalSequence)
        assertEquals("live preview", next.projects.single().desktopStreamText); assertEquals("4", next.workspaceRevision)
        val restored = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertEquals(next.turns, restored.turns); assertEquals(next.eventSequence, restored.eventSequence)
        assertNull(restored.projects.single().desktopStreamText); assertNull(restored.workspaceOwner)
    }
    @Test fun malformedCompletionOrCursorFailsWithoutReplacingPriorDisplay() {
        for (change in listOf<(JsonObject) -> Unit>({ it.remove("subscription_done") },
            { it.addProperty("subscription_done", "false") }, { it.addProperty("has_more", true) },
            { it["cursor"].asJsonObject.addProperty("next_sequence", 2) })) {
            val reducer = SubscriptionReducer(initial, project)
            val invalid = result(1, false).apply(change)
            assertTrue(runCatching { reducer.accept(listOf(event(0)), invalid) }.isFailure)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
    }
    @Test fun excessiveEmptyPagesCannotHoldClientForever() {
        val reducer = SubscriptionReducer(initial, project)
        repeat(31) { reducer.accept(emptyList(), result(0, false)) }
        assertTrue(runCatching { reducer.accept(emptyList(), result(0, false)) }.isFailure)
        reducer.accept(emptyList(), result(0, true))
        assertTrue(runCatching { reducer.accept(emptyList(), result(0, true)) }.isFailure)
        assertEquals(initial, reducer.projection)
    }
    @Test fun subscribeRequestHasStrictDurationScopeAndLimits() {
        fun request() = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","project_id":"project","session_id":"session","payload":{"type":"subscribe","cursor":{"session_id":"session","next_sequence":0},"limit":128,"duration_ms":3000}}""").asJsonObject
        FrameCodec.decode(request().toString().toByteArray())
        for (duration in listOf(0, 999, 5001, 65535)) {
            val value = request().apply { getAsJsonObject("payload").addProperty("duration_ms", duration) }
            assertTrue(runCatching { FrameCodec.decode(value.toString().toByteArray()) }.isFailure)
        }
        for (field in listOf("id", "project_id", "session_id")) {
            val value = request().apply { remove(field) }
            assertTrue(runCatching { FrameCodec.decode(value.toString().toByteArray()) }.isFailure)
        }
    }
    @Test fun transientDisplayUpdatesWithoutInventingTranscriptOrOfflineAuthority() {
        val reducer = SubscriptionReducer(initial, project)
        val update = transient().apply {
            val target = getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
            target.add("prompt_statuses", JsonParser.parseString("""[{"request_id":"prompt","project_id":"project","session_id":"session","state":"running","stream_text":"partial answer","stream_truncated":true}]"""))
            target.add("pending_approvals", JsonParser.parseString("""[{"request_id":"approval","project_id":"project","session_id":"session","turn_id":"turn","call_id":"call","tool":"write","side_effect":"workspace_write","origin":"agent_tool","digest":"${"c".repeat(64)}"}]"""))
        }
        val next = reducer.accept(emptyList(), update)
        assertEquals("live preview", next.projects.single().desktopStreamText)
        assertEquals("live terminal", next.projects.single().terminal?.text)
        assertEquals("partial answer", next.projects.single().promptStatuses.single().streamText)
        assertTrue(next.projects.single().promptStatuses.single().streamTruncated)
        assertEquals("approval", next.projects.single().pendingApprovals.single().requestId)
        assertEquals("0", next.eventSequence); assertEquals("0", next.journalSequence); assertEquals(initial.turns, next.turns)
        assertEquals("2", next.subtabs?.revision); assertFalse(next.projects.single().workspaceControl)
        val persisted = zenpi.state.ProjectionCacheCodec.encode(next)
        assertFalse(persisted.toString().contains("live preview")); assertFalse(persisted.toString().contains("live terminal"))
        val restored = zenpi.state.ProjectionCacheCodec.decode(persisted)
        assertEquals("0", restored.eventSequence); assertNull(restored.projects.single().terminal)
        assertTrue(restored.projects.single().pendingApprovals.isEmpty())
        assertNull(restored.projects.single().promptStatuses.single().streamText)
        assertFalse(restored.projects.single().promptStatuses.single().streamTruncated)
        assertEquals("unknown_outcome", restored.projects.single().promptStatuses.single().state)
    }
    @Test fun typedFailureSubscriptionKeepsCursorAndCacheWithoutWriteAuthority() {
        val update=transient().apply{
            getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
                .add("prompt_statuses",JsonParser.parseString("""[{"request_id":"failed-message","project_id":"project","session_id":"session","state":"failed","failure_code":"invalid_request"}]"""))
        }
        val next=SubscriptionReducer(initial,project).accept(emptyList(),update)
        assertEquals("invalid_request",next.projects.single().promptStatuses.single().failureCode)
        assertEquals(initial.turns,next.turns)
        assertEquals(initial.eventSequence,next.eventSequence)
        assertEquals(initial.journalSequence,next.journalSequence)
        val cached=zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertEquals(next.projects.single().promptStatuses,cached.projects.single().promptStatuses)
        assertFalse(cached.projects.single().promptControl)
        assertEquals(initial.eventSequence,cached.eventSequence)
    }
    @Test fun workerStatusSubscriptionDoesNotInventMasterTranscriptAndMalformedUpdateIsAtomic() {
        fun update() = transient().apply {
            val target = getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
            target.addProperty("worker_control", true)
            target.add("worker_statuses", JsonParser.parseString("""[{"request_id":"worktree-task","project_id":"project","session_id":"session","subtab_id":"${"a".repeat(64)}","state":"running"}]"""))
            target.getAsJsonArray("worker_statuses")[0].asJsonObject.addProperty("stream_text", "live worker preview")
        }
        val reducer = SubscriptionReducer(initial, project)
        val next = reducer.accept(emptyList(), update())
        assertTrue(next.projects.single().workerControl)
        assertEquals("running", next.projects.single().workerStatuses.single().state)
        assertEquals("live worker preview", next.projects.single().workerStatuses.single().streamText)
        assertFalse(next.projects.single().promptControl)
        assertEquals(initial.eventSequence, next.eventSequence); assertEquals(initial.journalSequence, next.journalSequence)
        assertEquals(initial.turns, next.turns)
        val completed = update().apply {
            getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
                .getAsJsonArray("worker_statuses")[0].asJsonObject.apply {
                    remove("stream_text")
                    addProperty("state", "succeeded"); addProperty("output_text", "completed task output"); addProperty("output_truncated", false)
                }
        }
        val done = reducer.accept(emptyList(), completed)
        assertEquals("completed task output", done.projects.single().workerStatuses.single().outputText)
        assertEquals(initial.turns, done.turns); assertEquals(initial.eventSequence, done.eventSequence)
        val invalid = update().apply {
            getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject
                .getAsJsonArray("worker_statuses")[0].asJsonObject.addProperty("session_id", "foreign")
        }
        assertTrue(runCatching { reducer.accept(emptyList(), invalid) }.isFailure)
        assertEquals(done, reducer.projection)
        val cached = zenpi.state.ProjectionCacheCodec.decode(zenpi.state.ProjectionCacheCodec.encode(next))
        assertFalse(cached.projects.single().workerControl); assertTrue(cached.projects.single().workerStatuses.isEmpty())
    }
    @Test fun transientOwnerRevisionCursorAndByteFailuresNeverReplacePriorDisplay() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it["cursor"].asJsonObject.addProperty("next_sequence", 1) },
            { it["cursor"].asJsonObject.addProperty("unexpected", 1) },
            { it["cursor"].asJsonObject.addProperty("next_sequence", "0") },
            { it.remove("cursor_scope") },
            { it.addProperty("cursor_scope", "session_journal") },
            { it["subscription_view"].asJsonObject["subtabs"].asJsonObject.addProperty("workspace_revision", 5) },
            { it["subscription_view"].asJsonObject["listing"].asJsonObject["projects"].asJsonArray[0].asJsonObject.addProperty("session_id", "foreign") },
            { it["subscription_view"].asJsonObject["listing"].asJsonObject.addProperty("active_project_id", "missing") },
            { it.addProperty("subscription_transcript", "false") },
            { it["subscription_view"].asJsonObject.addProperty("unexpected", true) },
            { it.addProperty("excess", "x".repeat(196_608)) })) {
            val reducer = SubscriptionReducer(initial, project)
            assertTrue(runCatching { reducer.accept(emptyList(), transient().apply(change)) }.isFailure)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
        val reducer = SubscriptionReducer(initial, project)
        assertTrue(runCatching { reducer.accept(listOf(event(0)), transient()) }.isFailure)
        assertEquals(initial, reducer.projection)
    }
    @Test fun staleTabViewCannotReplaceNewerViewOnTheSameSubscription() {
        val reducer = SubscriptionReducer(initial, project)
        val first = reducer.accept(emptyList(), transient())
        val stale = transient().apply {
            getAsJsonObject("subscription_view").getAsJsonObject("subtabs").addProperty("subtab_revision", 1)
            getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0]
                .asJsonObject.getAsJsonObject("desktop_terminal").addProperty("subtab_revision", 1)
        }
        assertTrue(runCatching { reducer.accept(emptyList(), stale) }.exceptionOrNull() is zenpi.state.StaleProjectionException)
        assertEquals(first, reducer.projection); assertFalse(reducer.done)
        val newer = transient().apply {
            getAsJsonObject("subscription_view").getAsJsonObject("subtabs").addProperty("subtab_revision", 3)
            getAsJsonObject("subscription_view").getAsJsonObject("listing").getAsJsonArray("projects")[0]
                .asJsonObject.getAsJsonObject("desktop_terminal").addProperty("subtab_revision", 3)
            addProperty("subscription_done", true)
        }
        assertEquals("3", reducer.accept(emptyList(), newer).subtabs?.revision)
        assertTrue(reducer.done)
    }
}
