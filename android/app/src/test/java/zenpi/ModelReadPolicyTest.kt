package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ModelReadPolicyTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val status = RemoteModelStatus("project", "session", "model", null, true, "1", null, "128000", "8192",
        catalog = RemoteModelCatalog("a".repeat(64), 0, 16, (0..15).map { RemoteModelChoice("model-%02d".format(it), "active", true) }))
    private val projection = HostProjection(listOf(project), "project", "session", "running", emptyList(), "1", false, modelStatus = status)
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection, modelOpen = true, modelStatus = status)

    @Test fun readOnlyRunningClientsCanBrowseAndErrorsRestartButNeverContinueStalePages() {
        assertTrue(ModelReadPolicy.browse(state)); assertTrue(ModelReadPolicy.navigation(state))
        assertFalse(project.modelControl); assertFalse(project.reasoningControl)
        for (changed in listOf(state.copy(modelError = "changed"), state.copy(modelOpen = false), state.copy(modelStatus = null),
            state.copy(modelStatus = status.copy(sessionId = "foreign")),
            state.copy(projection = projection.copy(modelStatus = status.copy(selectionRevision = "2"))),
            state.copy(projection = projection.copy(modelStatus = status.copy(catalog = status.catalog?.copy(snapshotId = "b".repeat(64))))))) {
            assertTrue(ModelReadPolicy.browse(changed)); assertFalse(ModelReadPolicy.navigation(changed))
        }
        assertTrue(ModelReadPolicy.navigation(state.copy(modelStatus = status.copy(registryBound = false),
            projection = projection.copy(modelStatus = status.copy(registryBound = false)))))
    }
    @Test fun scanningResetPendingOrUnlistedScopeRefuseReadsBeforeRequestAndMerge() {
        for (changed in listOf(state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
            state.copy(phase = PairingPhase.OFFLINE), state.copy(pendingRevoke = "revoke"), state.copy(modelLoading = true),
            state.copy(projection = null), state.copy(projection = projection.copy(sessionId = "foreign")),
            state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))))) {
            assertFalse(ModelReadPolicy.browse(changed)); assertFalse(ModelReadPolicy.navigation(changed))
        }
        for (changed in listOf(projection.copy(projects = emptyList()), projection.copy(sessionId = "foreign"),
            projection.copy(projects = listOf(project, project)), projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))))) {
            assertFalse(ModelStatusCodec.readable(changed)); assertFalse(ModelStatusCodec.sameScope(status, changed))
            assertTrue(runCatching { ModelStatusCodec.request(changed) }.isFailure)
            assertNull(ModelStatusCodec.newest(status, status, changed))
        }
        val writable = projection.copy(projects = listOf(project.copy(workspaceControl = true)), workspaceOwner = "b".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.create(writable, "project", "workspace_select")
        assertFalse(ModelReadPolicy.browse(state.copy(pendingWorkspace = command)))
    }
}
