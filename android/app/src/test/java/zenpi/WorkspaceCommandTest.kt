package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.state.ProjectionCacheCodec
import zenpi.pairing.*

class WorkspaceCommandTest {
    @Test fun primaryWorkspaceConstructionRejectsDuplicateIdentityBeforeCapabilityFiltering() {
        val command = WorkspaceCommand.create(value, project.id, "workspace_select")
        val ambiguous = value.copy(projects = listOf(project, project.copy(sessionId = "foreign", workspaceControl = false)))
        for (operation in listOf("workspace_select", "workspace_close", "workspace_open")) {
            assertTrue(runCatching { WorkspaceCommand.create(ambiguous, project.id, operation,
                if (operation == "workspace_open") "child" else null) }.isFailure)
        }
        val live = PairingState(phase = PairingPhase.CONNECTED, projection = ambiguous)
        assertFalse(WorkspaceDispatchPolicy.allows(live, true, true, command))
        val pending = live.copy(phase = PairingPhase.OFFLINE, pendingWorkspace = command)
        assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, command))
        assertFalse(WorkspaceDispatchPolicy.allows(pending.copy(scanInProgress = true), true, true, command))
        val other = project.copy(id = "other", sessionId = "other-session")
        val switched = WorkspaceCommand.create(value.copy(projects = listOf(project, other)), other.id, "workspace_select")
        assertEquals(other.sessionId, switched.envelope["session_id"].asString)
        assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
            projection = value.copy(projects = listOf(project, other))), true, true, switched))
    }
    @Test fun subtabConstructionAndDispatchRequireUniqueTargetAndMoveDestination() {
        val main = "c".repeat(64); val child = "b".repeat(64); val before = "d".repeat(64)
        val tabs = RemoteSubtabs(project.id, "3", main, listOf(RemoteSubtab(main, "main", "main", 1),
            RemoteSubtab(child, "child", "in_place", 1), RemoteSubtab(before, "before", "worktree", 1)), true, true, true, concurrencyControl = true)
        val live = value.copy(subtabs = tabs)
        val commands = listOf(WorkspaceCommand.subtab(live, "subtab_select", child),
            WorkspaceCommand.subtab(live, "subtab_rename", child, "renamed"),
            WorkspaceCommand.subtab(live, "subtab_move", child, before),
            WorkspaceCommand.subtab(live, "subtab_close", child), WorkspaceCommand.concurrency(live, child, 2))
        val duplicate = live.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs[1].copy(kind = "worktree")))
        for (operation in listOf("subtab_select", "subtab_rename", "subtab_move", "subtab_close")) {
            assertTrue(runCatching { WorkspaceCommand.subtab(duplicate, operation, child,
                when (operation) { "subtab_rename" -> "renamed"; "subtab_move" -> before; else -> null }) }.isFailure)
        }
        assertTrue(runCatching { WorkspaceCommand.concurrency(duplicate, child, 2) }.isFailure)
        for (command in commands) assertFalse(WorkspaceDispatchPolicy.allows(
            PairingState(phase = PairingPhase.CONNECTED, projection = duplicate), true, true, command))
        for (destination in listOf(live.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs[2])),
            live.copy(subtabs = tabs.copy(tabs = tabs.tabs.filter { it.id != before })))) {
            assertTrue(runCatching { WorkspaceCommand.subtab(destination, "subtab_move", child, before) }.isFailure)
            assertFalse(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED,
                projection = destination), true, true, commands[2]))
        }
        assertTrue(WorkspaceDispatchPolicy.allows(PairingState(phase = PairingPhase.CONNECTED, projection = live),
            true, true, WorkspaceCommand.subtab(live, "subtab_move", child)))
    }
    @Test fun connectedSubtabDispatchRevalidatesTargetRevisionAndIndividualCapabilities() {
        val main = "c".repeat(64); val directory = "b".repeat(64); val worktree = "d".repeat(64)
        val tabs = RemoteSubtabs("project", "3", main, listOf(RemoteSubtab(main, "main", "main", 1),
            RemoteSubtab(directory, "child", "in_place", 1), RemoteSubtab(worktree, "feature", "worktree", 1)), true, true, true)
        val live = value.copy(subtabs = tabs)
        val commands = listOf(WorkspaceCommand.subtab(live, "subtab_select", directory),
            WorkspaceCommand.subtab(live, "subtab_rename", directory, "renamed"),
            WorkspaceCommand.subtab(live, "subtab_move", directory, worktree),
            WorkspaceCommand.subtab(live, "subtab_close", directory), WorkspaceCommand.subtab(live, "subtab_close", worktree),
            WorkspaceCommand.directoryTab(live, "next", "next"), WorkspaceCommand.worktreeTab(live, "next-feature"))
        fun allows(command: WorkspaceCommand, current: HostProjection = live) = WorkspaceDispatchPolicy.allows(
            PairingState(phase = PairingPhase.CONNECTED, projection = current), true, true, command)
        for (command in commands) {
            assertTrue(allows(command))
            assertFalse(allows(command, live.copy(subtabs = tabs.copy(revision = "4"))))
            assertFalse(allows(command, live.copy(subtabs = tabs.copy(projectId = "foreign"))))
            assertFalse(allows(command, live.copy(projects = listOf(project.copy(workspaceControl = false)))))
        }
        for (command in commands.take(5)) assertFalse(allows(command, live.copy(subtabs = tabs.copy(control = false))))
        assertFalse(allows(commands[3], live.copy(subtabs = tabs.copy(directoryControl = false))))
        assertFalse(allows(commands[4], live.copy(subtabs = tabs.copy(worktreeControl = false))))
        assertFalse(allows(commands[5], live.copy(subtabs = tabs.copy(directoryControl = false))))
        assertFalse(allows(commands[6], live.copy(subtabs = tabs.copy(worktreeControl = false))))
        val removedTarget = live.copy(subtabs = tabs.copy(tabs = tabs.tabs.filter { it.id != directory }))
        for (command in commands.take(4)) assertFalse(allows(command, removedTarget))
        val removedBefore = live.copy(subtabs = tabs.copy(tabs = tabs.tabs.filter { it.id != worktree }))
        assertFalse(allows(commands[2], removedBefore))
        assertTrue(allows(commands[0], live.copy(subtabs = tabs.copy(activeId = worktree))))
    }
    @Test fun worktreeCreateAndCloseRequireLiveAuthorityAndNeverAcceptRemotePaths() {
        val id = "b".repeat(64)
        val live = value.copy(subtabs = RemoteSubtabs("project", "3", id,
            listOf(RemoteSubtab("c".repeat(64), "main", "main", 1), RemoteSubtab(id, "feature", "worktree", 1)), true, true, true))
        val add = WorkspaceCommand.worktreeTab(live, "new-feature")
        assertEquals(add.envelope, WorkspaceCommand.restore(add.envelope).envelope)
        val args = add.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals(setOf("kind", "name", "subtab_revision"), args.keySet())
        assertEquals("worktree", args["kind"].asString)
        for (name in listOf("", "-option", "../outside", "a/b", "bad..name", "bad name", "a".repeat(129)))
            assertTrue(name, runCatching { WorkspaceCommand.worktreeTab(live, name) }.isFailure)
        val path = add.envelope; path["payload"].asJsonObject["arguments"].asJsonObject.addProperty("cwd", "/outside")
        assertTrue(runCatching { WorkspaceCommand.restore(path) }.isFailure)
        val close = WorkspaceCommand.subtab(live, "subtab_close", id)
        assertEquals(close.envelope, WorkspaceCommand.restore(close.envelope).envelope)
        val denied = live.copy(subtabs = live.subtabs?.copy(worktreeControl = false))
        assertTrue(runCatching { WorkspaceCommand.worktreeTab(denied, "next") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.subtab(denied, "subtab_close", id) }.isFailure)
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.subtabs!!.worktreeControl)
        assertTrue(runCatching { WorkspaceCommand.worktreeTab(cached, "next") }.isFailure)
    }
    @Test fun steeringBindsLiveTargetAndRestoresExactPendingEnvelope() {
        val live = value.copy(projects = listOf(project.copy(promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("target", "project", "session", "running")))))
        val command = WorkspaceCommand.steer(live, "target", "use the existing implementation")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals("target", args["request_id"].asString)
        assertEquals("use the existing implementation", args["text"].asString)
        assertTrue(runCatching { WorkspaceCommand.steer(live, "foreign", "hello") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.steer(ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live)), "target", "hello") }.isFailure)
        for (text in listOf(" ", "!echo hello", "x\u0000y", "x".repeat(65537), "/skill:research", "  /review literal", "/help", "use @file.txt", "@", "@\"unfinished"))
            assertTrue(runCatching { WorkspaceCommand.steer(live, "target", text) }.isFailure)
        for (text in listOf("email a@example.invalid", "mention (@word)", "keep /path literal", "inline !command"))
            assertEquals(text, WorkspaceCommand.steer(live, "target", text).envelope["payload"].asJsonObject["arguments"].asJsonObject["text"].asString)
        val legacy = command.envelope
        legacy["payload"].asJsonObject["arguments"].asJsonObject.addProperty("text", "/review literal")
        assertEquals(legacy, WorkspaceCommand.restore(legacy).envelope)
        val invalid = command.envelope
        invalid["payload"].asJsonObject["arguments"].asJsonObject.addProperty("request_id", "bad\nID")
        assertTrue(runCatching { WorkspaceCommand.restore(invalid) }.isFailure)
    }
    @Test fun cancellationCapturesOnlyAnExplicitLivePromptTarget() {
        val live = value.copy(projects = listOf(project.copy(promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("target", "project", "session", "running")))))
        val command = WorkspaceCommand.cancelPrompt(live, "target")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertEquals("target", command.envelope["payload"].asJsonObject["arguments"].asJsonObject["request_id"].asString)
        assertTrue(runCatching { WorkspaceCommand.cancelPrompt(live, "foreign") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.cancelPrompt(ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live)), "target") }.isFailure)
    }
    @Test fun restoredCancellationCannotRetainMalformedTargetButPreservesValidExactRetry() {
        val live = value.copy(projects = listOf(project.copy(promptControl = true,
            promptStatuses = listOf(RemotePromptStatus("target", "project", "session", "running")))))
        val command = WorkspaceCommand.cancelPrompt(live, "target")
        for (target in listOf("", " ", "bad\nID", "bad\u0000ID", "bad\u007fID", "x".repeat(129), "界".repeat(43))) {
            val malformed = command.envelope
            malformed.getAsJsonObject("payload").getAsJsonObject("arguments").addProperty("request_id", target)
            assertTrue(target, runCatching { WorkspaceCommand.restore(malformed) }.exceptionOrNull() is WireException)
        }
        for (target in listOf("target", "request-with spaces", "界".repeat(42), "x".repeat(128))) {
            val valid = command.envelope
            valid.getAsJsonObject("payload").getAsJsonObject("arguments").addProperty("request_id", target)
            val restored = WorkspaceCommand.restore(valid)
            assertEquals(valid, restored.envelope)
            val pending = PairingState(phase = PairingPhase.OFFLINE, pendingWorkspace = restored)
            assertTrue(WorkspaceDispatchPolicy.allows(pending, true, true, WorkspaceCommand.restore(valid)))
            assertFalse(WorkspaceDispatchPolicy.allows(pending, false, true, restored))
        }
    }
    @Test fun promptNeedsLiveAuthorityAndPersistsExactTextWithoutLocalShellRoute() {
        val live = value.copy(projects = listOf(project.copy(promptControl = true)))
        val command = WorkspaceCommand.prompt(live, "hello\nworld")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        assertEquals("hello\nworld", command.envelope["payload"].asJsonObject["arguments"].asJsonObject["text"].asString)
        assertTrue(runCatching { WorkspaceCommand.prompt(value, "hello") }.isFailure)
        for (text in listOf(" ", "!echo hello", "  !echo hello", "x".repeat(65537)))
            assertTrue(runCatching { WorkspaceCommand.prompt(live, text) }.isFailure)
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertFalse(cached.projects.single().promptControl)
        assertTrue(runCatching { WorkspaceCommand.prompt(cached, "hello") }.isFailure)
    }
    @Test fun directoryCreateAndCloseHaveStrictPendingEnvelopeWithoutGitAuthority() {
        val id = "b".repeat(64)
        val live = value.copy(subtabs = RemoteSubtabs("project", "3", id,
            listOf(RemoteSubtab("c".repeat(64), "main", "main", 1), RemoteSubtab(id, "child", "in_place", 1)), true, true))
        val add = WorkspaceCommand.directoryTab(live, "other", "other")
        assertEquals(add.envelope, WorkspaceCommand.restore(add.envelope).envelope)
        val args = add.envelope["payload"].asJsonObject["arguments"].asJsonObject
        assertEquals("in_place", args["kind"].asString); assertEquals("3", args["subtab_revision"].toString())
        val git = add.envelope; git["payload"].asJsonObject["arguments"].asJsonObject.addProperty("kind", "worktree")
        assertTrue(runCatching { WorkspaceCommand.restore(git) }.isFailure)
        val close = WorkspaceCommand.subtab(live, "subtab_close", id)
        assertEquals(close.envelope, WorkspaceCommand.restore(close.envelope).envelope)
        assertTrue(runCatching { WorkspaceCommand.subtab(live, "subtab_close", "c".repeat(64)) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.directoryTab(live.copy(subtabs = live.subtabs?.copy(directoryControl = false)), "other", "other") }.isFailure)
    }
    @Test fun subtabCommandsRetainExactIdentityRevisionAndDropAuthorityOnCacheRestore() {
        val id = "b".repeat(64)
        val live = value.copy(subtabs = RemoteSubtabs("project", "7", id, listOf(RemoteSubtab(id, "main", "main", 1)), true))
        for (operation in listOf("subtab_select", "subtab_rename", "subtab_move")) {
            val command = WorkspaceCommand.subtab(live, operation, id, if (operation == "subtab_rename") "renamed" else null)
            assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
            val args = command.envelope["payload"].asJsonObject["arguments"].asJsonObject
            assertEquals(id, args["subtab_id"].asString); assertEquals("7", args["subtab_revision"].toString())
            val bad = command.envelope; bad["payload"].asJsonObject["arguments"].asJsonObject.addProperty("subtab_revision", "7")
            assertTrue(runCatching { WorkspaceCommand.restore(bad) }.isFailure)
        }
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(live))
        assertEquals(live.subtabs?.copy(control = false, directoryControl = false), cached.subtabs)
        assertTrue(runCatching { WorkspaceCommand.subtab(cached, "subtab_select", id) }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.subtab(live, "subtab_rename", id, " bad ") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.subtab(live, "subtab_move", id, "index-1") }.isFailure)
    }
    private val project = RemoteProject("project", "Workspace", "session", true)
    private val value = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false,
        "0", workspaceOwner = "a".repeat(64), workspaceRevision = "18446744073709551615")
    @Test fun commandCapturesImmutableOwnerRevisionIdentityAndArguments() {
        val command = WorkspaceCommand.create(value, "project", "workspace_open", "child")
        val original = command.envelope.deepCopy()
        assertEquals("child", original["payload"].asJsonObject["arguments"].asJsonObject["cwd"].asString)
        assertEquals("session", original["session_id"].asString)
        assertEquals("18446744073709551615", original["expected_revision"].toString())
        assertEquals(command.envelope, original)
        command.envelope.addProperty("id", "tampered")
        assertEquals(command.envelope, original)
        assertNotEquals(command.envelope["id"], WorkspaceCommand.create(value, "project", "workspace_open", "child").envelope["id"])
    }
    @Test fun staleOfflineCacheNeverRestoresMutationAuthority() {
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(value))
        assertFalse(cached.projects.single().workspaceControl)
        assertNull(cached.workspaceOwner); assertNull(cached.workspaceRevision)
        assertTrue(runCatching { WorkspaceCommand.create(cached, "project", "workspace_select") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.create(value, "foreign", "workspace_select") }.isFailure)
        assertTrue(runCatching { WorkspaceCommand.create(value, "project", "workspace_open", " ") }.isFailure)
    }
    @Test fun pendingCommandRestoreIsStrictAndPreservesTheExactRequestId() {
        val command = WorkspaceCommand.create(value, "project", "workspace_select")
        assertEquals(command.envelope, WorkspaceCommand.restore(command.envelope).envelope)
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("extra", true) }, { it["payload"].asJsonObject.addProperty("operation", "prompt") },
            { it["payload"].asJsonObject["arguments"].asJsonObject.addProperty("cwd", "child") },
            { it.addProperty("required_owner", "foreign") }, { it.addProperty("expected_revision", "3") })) {
            val malformed = command.envelope; change(malformed)
            assertTrue(runCatching { WorkspaceCommand.restore(malformed) }.isFailure)
        }
    }
}
