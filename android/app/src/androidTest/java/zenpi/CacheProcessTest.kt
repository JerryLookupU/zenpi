package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.transport.*

/** Run seed and restore in separate instrumentation processes, with force-stop between. */
class CacheProcessTest {
    @Test fun encryptedDisplayAndCursorSurviveIndependentProcess() {
        val args = InstrumentationRegistry.getArguments()
        val phase = args.getString("cache_phase") ?: return
        val namespace = requireNotNull(args.getString("cache_namespace"))
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val store = PairedHostStore(context, namespace)
        val keyAlias = "zenpi.cache-process.$namespace"
        val host = PairedHost("127.0.0.1", 7777, "a".repeat(64), DeviceIdentity.loadOrCreate(keyAlias).deviceId)
        val project = RemoteProject("project", "Workspace", "session")
        val projection = HostProjection(listOf(project), "project", "session", "idle",
            listOf(RemoteTurn("turn", "assistant", "CACHE_PROCESS_DISPLAY")), "42", false, "7",
            subtabs = RemoteSubtabs("project", "9", "c".repeat(64), listOf(
                RemoteSubtab("a".repeat(64), "main", "main", 1), RemoteSubtab("c".repeat(64), "CACHE_DIRECTORY", "in_place", 64),
                RemoteSubtab("b".repeat(64), "CACHE_FEATURE", "worktree", 2)), false))
        if (phase == "seed") {
            store.wipe(); store.save(host); store.saveProjection(host, projection.copy(
                projects = listOf(project.copy(workspaceControl = true)), workspaceOwner = "b".repeat(64), workspaceRevision = "3",
                subtabs = projection.subtabs?.copy(control = true, directoryControl = true)))
            assertEquals(projection, store.loadState()?.projection)
            val command = WorkspaceCommand.create(projection.copy(projects = listOf(project.copy(workspaceControl = true)),
                workspaceOwner = "b".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
            store.beginWorkspace(host, command)
            store.saveProjection(host, projection)
            assertEquals(command.envelope, store.loadState()?.pendingWorkspace?.envelope)
            assertTrue(runCatching { store.save(host) }.isFailure)
            assertTrue(runCatching { store.beginWorkspace(host, WorkspaceCommand.create(projection.copy(projects = listOf(project.copy(workspaceControl = true)),
                workspaceOwner = "b".repeat(64), workspaceRevision = "3"), "project", "workspace_select")) }.isFailure)
        } else {
            try {
                assertEquals("restore", phase)
                val firstPid = requireNotNull(args.getString("previous_pid")).toInt()
                assertNotEquals("Restore must use another OS process", firstPid, android.os.Process.myPid())
                assertEquals(host, store.loadState()?.host)
                assertEquals(projection, store.loadState()?.projection)
                assertFalse(requireNotNull(store.loadState()?.projection?.subtabs).control)
                assertFalse(requireNotNull(store.loadState()?.projection?.subtabs).directoryControl)
                assertTrue(runCatching { WorkspaceCommand.subtab(requireNotNull(store.loadState()?.projection), "subtab_select", "b".repeat(64)) }.isFailure)
                val pending = requireNotNull(store.loadState()?.pendingWorkspace)
                assertEquals("workspace_select", pending.envelope["payload"].asJsonObject["operation"].asString)
                assertEquals("3", pending.envelope["expected_revision"].toString())
                val wrong = WorkspaceCommand.create(projection.copy(projects = listOf(project.copy(workspaceControl = true)),
                    workspaceOwner = "b".repeat(64), workspaceRevision = "3"), "project", "workspace_select")
                assertTrue(runCatching { store.finishWorkspace(host, wrong) }.isFailure)
                assertTrue(runCatching { store.beginWorkspace(host.copy(port = 7778), pending) }.isFailure)
                assertEquals(pending.envelope, store.loadState()?.pendingWorkspace?.envelope)
                store.finishWorkspace(host, pending, projection)
                assertNull(store.loadState()?.pendingWorkspace)
                val keyStore = java.security.KeyStore.getInstance("AndroidKeyStore").apply { load(null) }
                assertNull(keyStore.getKey(keyAlias, null).encoded)
            } finally { store.wipe(); DeviceIdentity.loadOrCreate(keyAlias).wipe() }
            assertNull(store.loadState())
        }
        InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("cache_process_pid", android.os.Process.myPid().toString())
            putString("cache_process_phase", phase)
        })
    }
}
