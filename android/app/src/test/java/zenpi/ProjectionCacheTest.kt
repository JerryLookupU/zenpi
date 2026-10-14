package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.state.ProjectionCacheCodec
import zenpi.transport.*

class ProjectionCacheTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val value = HostProjection(listOf(project), "project", "session", "idle",
        listOf(RemoteTurn("one", "assistant", "display")), "18446744073709551615", true, "7")
    @Test fun terminalTypedFailureSurvivesCacheWithoutRestoringLiveAuthority() {
        val failed=RemotePromptStatus("request","project","session","failed",failureCode="resource_limit")
        val live=value.copy(projects=listOf(project.copy(promptControl=true,promptStatuses=listOf(failed))))
        val restored=ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertEquals(failed,restored.projects.single().promptStatuses.single())
        assertFalse(restored.projects.single().promptControl)
    }
    @Test fun displayAndCursorRoundtripAsOneStrictRecord() {
        assertEquals(value, ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(value)))
        assertTrue(runCatching { ProjectionCacheCodec.encode(value.copy(eventSequence = null)) }.isFailure)
        assertTrue(runCatching { ProjectionCacheCodec.encode(value.copy(sessionId = "foreign")) }.isFailure)
    }
    @Test fun impossibleCursorCannotEnterOrRestoreAnyCacheVersion() {
        assertTrue(runCatching { ProjectionCacheCodec.encode(value.copy(journalSequence = "6")) }.exceptionOrNull() is WireException)
        for (version in 1..4) {
            val encoded = ProjectionCacheCodec.encode(value).apply {
                addProperty("version", version)
                if (version == 1) remove("subtabs")
                if (version < 3) getAsJsonObject("listing").getAsJsonArray("projects").forEach { it.asJsonObject.remove("prompt_statuses") }
                getAsJsonObject("snapshot").addProperty("journal_next_sequence", 6)
            }
            val unchanged = encoded.deepCopy()
            assertTrue(runCatching { ProjectionCacheCodec.decode(encoded) }.exceptionOrNull() is WireException)
            assertEquals(unchanged, encoded)
        }
    }
    @Test fun durableDisplayComparisonUsesTheCacheContractNotTransientWorkspaceAuthority() {
        val live = value.copy(projects = listOf(project.copy(promptControl = true, sessionControl = true,
            sessionName = "Development", desktopStreamText = "transient")), workspaceOwner = "d".repeat(64),
            workspaceRevision = "9", subtabs = tabs)
        val encoded = ProjectionCacheCodec.encode(live)
        val cached = ProjectionCacheCodec.decode(encoded)
        assertNull(cached.workspaceOwner); assertNull(cached.workspaceRevision)
        assertNull(cached.projects.single().sessionName)
        assertFalse(cached.projects.single().sessionControl)
        assertEquals(encoded, ProjectionCacheCodec.encode(cached))
        assertNotEquals(encoded, ProjectionCacheCodec.encode(live.copy(eventSequence = "8")))
        assertNotEquals(encoded, ProjectionCacheCodec.encode(live.copy(turns = emptyList())))
        val replacement = live.copy(sessionId = "new", projects = listOf(project.copy(sessionId = "new")),
            journalSequence = "0", eventSequence = "0")
        assertNotEquals(encoded, ProjectionCacheCodec.encode(replacement))
        assertEquals(encoded, ProjectionCacheCodec.encode(live.copy(workspaceRevision = "10")))
    }
    @Test fun truncatedDurableMessageRetainsFlagAndCursorAcrossCacheRestore() {
        val live = value.copy(turns = listOf(RemoteTurn("large", "assistant", "visible prefix", true)), truncated = true)
        assertEquals(live, ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live)))
        val invalid = ProjectionCacheCodec.encode(live)
        invalid["snapshot"].asJsonObject["transcript"].asJsonArray[0].asJsonObject.addProperty("content_truncated", "true")
        assertTrue(runCatching { ProjectionCacheCodec.decode(invalid) }.isFailure)
    }
    @Test fun corruptionUnknownVersionAndCursorCoercionRefused() {
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("version", 99) }, { it.addProperty("unexpected", true) },
            { it["snapshot"].asJsonObject["event_cursor"].asJsonObject.addProperty("next_sequence", "7") },
            { it["snapshot"].asJsonObject.addProperty("session_id", "foreign") })) {
            val encoded = ProjectionCacheCodec.encode(value); change(encoded)
            assertTrue(runCatching { ProjectionCacheCodec.decode(encoded) }.isFailure)
        }
    }
    @Test fun excessDataDoesNotProduceAnAdvancedCursorWithoutItsDisplay() {
        assertTrue(runCatching { ProjectionCacheCodec.encode(value.copy(turns = listOf(RemoteTurn("huge", "assistant", "x".repeat(ProjectionCacheCodec.MAX_BYTES))))) }.isFailure)
        assertEquals("7", value.eventSequence)
    }
    private val tabs = RemoteSubtabs("project", "18446744073709551615", "c".repeat(64), listOf(
        RemoteSubtab("a".repeat(64), "main", "main", 1), RemoteSubtab("c".repeat(64), "directory", "in_place", 64),
        RemoteSubtab("b".repeat(64), "feature", "worktree", 2)), true, true)
    @Test fun secondaryOrderSelectionAndQuotaSurviveWithoutAuthority() {
        val live = value.copy(projects = listOf(project.copy(workspaceControl = true)), workspaceOwner = "d".repeat(64),
            workspaceRevision = "9", subtabs = tabs)
        val encoded = ProjectionCacheCodec.encode(live)
        val restored = ProjectionCacheCodec.decode(encoded)
        assertEquals(tabs.copy(control = false, directoryControl = false), restored.subtabs)
        assertFalse(restored.projects.single().workspaceControl)
        assertNull(restored.workspaceOwner); assertNull(restored.workspaceRevision)
        assertEquals(encoded, ProjectionCacheCodec.encode(restored))
        assertTrue(runCatching { WorkspaceCommand.subtab(restored, "subtab_select", tabs.activeId) }.isFailure)
        assertFalse(encoded.toString().contains("subtab_control"))
    }
    @Test fun legacyDisplayRecordUpgradesWithoutInventingSecondaryTabs() {
        val legacy = ProjectionCacheCodec.encode(value).apply { addProperty("version", 1); remove("subtabs")
            getAsJsonObject("listing").getAsJsonArray("projects").forEach { it.asJsonObject.remove("prompt_statuses") } }
        assertEquals(value.copy(turns = value.turns.map { it.copy(toolCallsOmitted = true) }), ProjectionCacheCodec.decode(legacy))
        val upgraded = ProjectionCacheCodec.encode(ProjectionCacheCodec.decode(legacy))
        assertEquals("4", upgraded["version"].toString()); assertTrue(upgraded["subtabs"].isJsonNull)
    }
    @Test fun promptStatusCacheKeepsTerminalOutcomesButNeverClaimsRunningOffline() {
        val statuses = listOf("accepted", "running", "succeeded", "failed", "cancelled", "unknown_outcome").mapIndexed { index, state ->
            RemotePromptStatus("request-$index", "project", "session", state)
        }
        val live = value.copy(projects = listOf(project.copy(promptControl = true, promptStatuses = statuses)))
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertEquals(listOf("unknown_outcome", "unknown_outcome", "succeeded", "failed", "cancelled", "unknown_outcome"), cached.projects.single().promptStatuses.map { it.state })
        assertFalse(cached.projects.single().promptControl)
        val malformed = ProjectionCacheCodec.encode(live)
        malformed["listing"].asJsonObject["projects"].asJsonArray[0].asJsonObject["prompt_statuses"].asJsonArray[0].asJsonObject.addProperty("state", "running")
        assertTrue(runCatching { ProjectionCacheCodec.decode(malformed) }.isFailure)
    }
    @Test fun versionTwoDisplayMigratesWithNoInventedPromptStatus() {
        val legacy = ProjectionCacheCodec.encode(value.copy(subtabs = tabs)).apply {
            addProperty("version", 2)
            getAsJsonObject("listing").getAsJsonArray("projects").forEach { it.asJsonObject.remove("prompt_statuses") }
        }
        val restored = ProjectionCacheCodec.decode(legacy)
        assertEquals(emptyList<RemotePromptStatus>(), restored.projects.single().promptStatuses)
        assertEquals(tabs.copy(control = false, directoryControl = false), restored.subtabs)
        assertEquals("4", ProjectionCacheCodec.encode(restored)["version"].toString())
    }
    @Test fun legacyMigrationPreservesItsInputAndRejectsAuthorityAndUnknownSchema() {
        for (version in listOf(1,2)) {
            val legacy=ProjectionCacheCodec.encode(value.copy(subtabs=if(version==2)tabs else null)).apply {
                addProperty("version",version)
                if(version==1)remove("subtabs")
                getAsJsonObject("listing").getAsJsonArray("projects").forEach{it.asJsonObject.remove("prompt_statuses")}
            }
            val original=legacy.deepCopy()
            val restored=ProjectionCacheCodec.decode(legacy)
            assertEquals(original,legacy)
            assertEquals(value.turns.map { it.copy(toolCallsOmitted = true) },restored.turns);assertEquals(value.eventSequence,restored.eventSequence)
            assertNull(restored.workspaceOwner);assertNull(restored.modelStatus)
            val upgraded=ProjectionCacheCodec.encode(restored)
            assertEquals("4",upgraded["version"].toString());assertEquals(original,legacy)
            assertEquals(restored,ProjectionCacheCodec.decode(upgraded))
            for(change in listOf<(com.google.gson.JsonObject)->Unit>(
                {it.addProperty("version",5)},
                {it.getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject.addProperty("model_control",true)},
                {it.getAsJsonObject("listing").getAsJsonArray("projects")[0].asJsonObject.addProperty("prompt_control",true)},
                {it.getAsJsonObject("snapshot").getAsJsonObject("event_cursor").addProperty("next_sequence","7")})) {
                val corrupted=legacy.deepCopy().apply(change);val before=corrupted.deepCopy()
                assertTrue(runCatching{ProjectionCacheCodec.decode(corrupted)}.isFailure)
                assertEquals(before,corrupted)
            }
        }
    }
    @Test fun cachedSecondaryCorruptionAndPathOrCapabilityInjectionFailClosed() {
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("subtab_revision", "4") },
            { it.addProperty("active_subtab_id", "e".repeat(64)) }, { it.addProperty("control", true) },
            { it["tabs"].asJsonArray[0].asJsonObject.addProperty("root", "/private/project") },
            { it["tabs"].asJsonArray[1].asJsonObject.addProperty("concurrency", 65) },
            { it["tabs"].asJsonArray[1].asJsonObject.addProperty("kind", "main") },
            { it["tabs"].asJsonArray[1].asJsonObject.addProperty("name", "bad\nname") },
            { it["tabs"].asJsonArray.add(it["tabs"].asJsonArray[1].deepCopy()) },
            { it.add("tabs", com.google.gson.JsonArray()) })) {
            val encoded = ProjectionCacheCodec.encode(value.copy(subtabs = tabs))
            change(encoded["subtabs"].asJsonObject)
            assertTrue(runCatching { ProjectionCacheCodec.decode(encoded) }.isFailure)
        }
        assertTrue(runCatching { ProjectionCacheCodec.encode(value.copy(subtabs = tabs.copy(projectId = "foreign"))) }.isFailure)
    }
    @Test fun cachedAuthorityAndUnknownNestedFieldsAreNeverAccepted() {
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it["listing"].asJsonObject["projects"].asJsonArray[0].asJsonObject.addProperty("workspace_control", true) },
            { it["listing"].asJsonObject.addProperty("required_owner", "a".repeat(64)) },
            { it["snapshot"].asJsonObject.addProperty("root", "/private/project") },
            { it["snapshot"].asJsonObject["event_cursor"].asJsonObject.addProperty("unexpected", 1) },
            { it["snapshot"].asJsonObject["transcript"].asJsonArray[0].asJsonObject.addProperty("unexpected", 1) })) {
            val encoded = ProjectionCacheCodec.encode(value); change(encoded)
            assertTrue(runCatching { ProjectionCacheCodec.decode(encoded) }.isFailure)
        }
    }
    @Test fun liveTerminalPreviewAndStreamingTextNeverSurviveCacheRestore() {
        val liveProject = project.copy(desktopStreamText = "desktop transient output",
            terminal = RemoteTerminal("project", "session", tabs.activeId, tabs.revision, "terminal transient output", false),
            promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running", "prompt transient output", true)))
        val encoded = ProjectionCacheCodec.encode(value.copy(projects = listOf(liveProject), subtabs = tabs))
        val restored = ProjectionCacheCodec.decode(encoded)
        assertNull(restored.projects.single().terminal); assertNull(restored.projects.single().desktopStreamText)
        assertNull(restored.projects.single().promptStatuses.single().streamText)
        assertFalse(restored.projects.single().promptStatuses.single().streamTruncated)
        assertFalse(encoded.toString().contains("stream_truncated"))
        assertEquals("unknown_outcome", restored.projects.single().promptStatuses.single().state)
        assertEquals(value.eventSequence, restored.eventSequence)
        assertFalse(encoded.toString().contains("transient output"))
    }
    @Test fun individuallyValidViewsCombineIntoReadableCacheWithSelectedStatusPriority() {
        val projects = (0 until 64).map { index ->
            RemoteProject("project-$index", "Workspace", "session-$index", promptStatuses = (0 until 9).map { status ->
                RemotePromptStatus("request-$index-$status", "project-$index", "session-$index", "failed", failureCode = "internal_error")
            })
        }
        val turns = (0 until 64).map { RemoteTurn("turn-$it", "assistant", "answer") }
        val live = value.copy(projects = projects, selectedId = projects.last().id, sessionId = projects.last().sessionId, turns = turns)
        val unbounded = com.google.gson.JsonObject().apply {
            addProperty("mutations_available", false); addProperty("active_project_id", projects.last().id)
            add("projects", com.google.gson.JsonArray().apply { projects.forEach { project -> add(com.google.gson.JsonObject().apply {
                addProperty("id", project.id); addProperty("title", project.title); addProperty("session_id", project.sessionId)
                addProperty("workspace_control", false)
                add("prompt_statuses", com.google.gson.JsonArray().apply { project.promptStatuses.forEach { status -> add(com.google.gson.JsonObject().apply {
                    addProperty("request_id", status.requestId); addProperty("project_id", status.projectId)
                    addProperty("session_id", status.sessionId); addProperty("state", status.state); addProperty("failure_code", status.failureCode)
                }) } })
            }) } })
        }
        fun envelope(result: com.google.gson.JsonObject) = com.google.gson.JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", "local-metadata")
            addProperty("execution_state", "succeeded")
            add("payload", com.google.gson.JsonObject().apply { addProperty("type", "response"); add("result", result) })
        }
        FrameCodec.decode(envelope(unbounded).toString().toByteArray())
        val cache = ProjectionCacheCodec.encode(live)
        val old = cache.deepCopy().apply { add("listing", unbounded) }
        val record = com.google.gson.JsonObject().apply {
            addProperty("host", "192.168.1.2"); addProperty("port", 7777)
            addProperty("fingerprint", "a".repeat(64)); addProperty("device_id", "b".repeat(64)); add("projection", old)
        }
        assertTrue(runCatching { FrameCodec.decode(envelope(record).toString().toByteArray()) }.isFailure)
        record.add("projection", cache)
        FrameCodec.decode(envelope(record).toString().toByteArray())
        val restored = ProjectionCacheCodec.decode(cache)
        assertEquals(ProjectionCacheCodec.MAX_PROMPT_STATUSES, restored.projects.sumOf { it.promptStatuses.size })
        assertEquals(projects.last().promptStatuses, restored.projects.last().promptStatuses)
        assertEquals(projects.map { it.id }, restored.projects.map { it.id })
        assertEquals(turns, restored.turns); assertEquals(live.eventSequence, restored.eventSequence)
        assertEquals(cache, ProjectionCacheCodec.encode(restored))
        assertEquals(576, live.projects.sumOf { it.promptStatuses.size })
    }
    @Test fun byteBudgetTrimsLowestPriorityStatusHistoryWithoutDiscardingMessageOrCursor() {
        val projects = (0 until 16).map { index ->
            RemoteProject("project-$index", "Workspace", "session-$index", promptStatuses = (0 until 16).map { status ->
                RemotePromptStatus("request-$index-$status-" + "x".repeat(100), "project-$index", "session-$index", "failed", failureCode = "internal_error")
            })
        }
        val turns = (0 until 8).map { RemoteTurn("turn-$it", "assistant", "x".repeat(12 * 1024 - 128)) }
        val live = value.copy(projects = projects, selectedId = projects.last().id, sessionId = projects.last().sessionId, turns = turns)
        val encoded = ProjectionCacheCodec.encode(live)
        val restored = ProjectionCacheCodec.decode(encoded)
        assertTrue(encoded.toString().toByteArray(Charsets.UTF_8).size <= ProjectionCacheCodec.MAX_BYTES)
        assertTrue(restored.projects.sumOf { it.promptStatuses.size } < ProjectionCacheCodec.MAX_PROMPT_STATUSES)
        assertEquals(projects.last().promptStatuses, restored.projects.last().promptStatuses)
        assertTrue(restored.projects.dropLast(1).any { it.promptStatuses.isEmpty() })
        assertEquals(projects.map { it.id }, restored.projects.map { it.id })
        assertEquals(turns, restored.turns); assertEquals(live.eventSequence, restored.eventSequence)
        assertEquals(live.journalSequence, restored.journalSequence)
        assertEquals(encoded, ProjectionCacheCodec.encode(restored))
        assertEquals(256, live.projects.sumOf { it.promptStatuses.size })
        assertTrue(ProjectionCacheCodec.encode(live.copy(turns = emptyList())).toString().toByteArray().size < ProjectionCacheCodec.MAX_BYTES)
    }
}
