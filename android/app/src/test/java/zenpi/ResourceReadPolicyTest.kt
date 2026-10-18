package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ResourceReadPolicyTest {
    private val project = RemoteProject("project", "Project", "session")
    private val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0")
    private val state = PairingState(phase = PairingPhase.CONNECTED, projection = projection,
        resources = ResourceCatalog("project", "session", "1", emptyList(), false, "a".repeat(64)))

    @Test fun readOnlyDeviceCanBrowseAndPageButCannotInsertOrBorrowReloadAuthority() {
        assertTrue(ResourceReadPolicy.browse(state)); assertTrue(ResourceReadPolicy.navigation(state))
        assertFalse(ResourceReadPolicy.insertion(state))
        assertTrue(runCatching { WorkspaceCommand.reloadResources(projection, requireNotNull(state.resources)) }.isFailure)
        val controlling = state.copy(projection = projection.copy(projects = listOf(project.copy(promptControl = true))))
        assertTrue(ResourceReadPolicy.insertion(controlling))
        val running = controlling.copy(projection = requireNotNull(controlling.projection).copy(projects = listOf(
            project.copy(promptControl = true, promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running"))))))
        assertTrue(ResourceReadPolicy.navigation(running)); assertFalse(ResourceReadPolicy.insertion(running))
    }
    @Test fun readAdmissionRequiresUniqueCurrentScopeAndLifecycleButFailedCatalogCanRefresh() {
        for (changed in listOf(state.copy(phase = PairingPhase.OFFLINE), state.copy(scanInProgress = true),
            state.copy(resetRequired = true), state.copy(resetting = true), state.copy(resourcesLoading = true),
            state.copy(pendingRevoke = "revoke"), state.copy(projection = null),
            state.copy(projection = projection.copy(projects = emptyList())),
            state.copy(projection = projection.copy(projects = listOf(project, project))),
            state.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            state.copy(projection = projection.copy(sessionId = "other")))) {
            assertFalse(ResourceReadPolicy.browse(changed)); assertFalse(ResourceReadPolicy.navigation(changed))
            assertFalse(ResourceReadPolicy.insertion(changed))
        }
        val failed = state.copy(resourcesError = "unavailable", resources = null)
        assertTrue(ResourceReadPolicy.browse(failed)); assertFalse(ResourceReadPolicy.navigation(failed))
    }
    @Test fun initialCatalogRequestsRefuseUnlistedAndAmbiguousSelectedProjects() {
        for (changed in listOf(projection.copy(projects = emptyList()), projection.copy(sessionId = "foreign"),
            projection.copy(projects = listOf(project, project)), projection.copy(projects = listOf(project, project.copy(sessionId = "foreign"))))) {
            assertFalse(ResourceCatalogCodec.readable(changed))
            assertFalse(ResourceCatalogCodec.sameScope(requireNotNull(state.resources), changed))
            assertTrue(runCatching { ResourceCatalogCodec.request(changed) }.isFailure)
        }
    }
    @Test fun staleCatalogAndPendingWritesCannotNavigateOrInsert() {
        for (catalog in listOf(requireNotNull(state.resources).copy(projectId = "other"),
            requireNotNull(state.resources).copy(sessionId = "other"))) {
            assertTrue(ResourceReadPolicy.browse(state.copy(resources = catalog)))
            assertFalse(ResourceReadPolicy.navigation(state.copy(resources = catalog)))
        }
        val command = WorkspaceCommand.prompt(projection.copy(projects = listOf(project.copy(promptControl = true)),
            workspaceOwner = "b".repeat(64), workspaceRevision = "1"), "pending")
        assertFalse(ResourceReadPolicy.browse(state.copy(pendingWorkspace = command)))
        assertFalse(ResourceReadPolicy.navigation(state.copy(pendingWorkspace = command)))
    }
}
