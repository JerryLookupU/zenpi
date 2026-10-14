package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class ResourceCatalogScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun reloadRequiresAdvertisedControlAndIsDisabledOfflineOrDuringPendingWrite() {
        val project = RemoteProject("project", "Project", "session", promptControl = true, resourcesReloadControl = true)
        val projection = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "1", false, "0",
            workspaceOwner = "a".repeat(64), workspaceRevision = "1")
        val catalog = ResourceCatalog("project", "session", "2", emptyList(), false, "b".repeat(64))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            resourcesOpen = true, resources = catalog))
        var reloads = 0
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onReloadResources = { reloads++ }) } }
        compose.onNodeWithContentDescription("Reload host resources").assertIsEnabled().performClick()
        assertEquals(1, reloads)
        compose.runOnIdle { state.value = state.value.copy(resources = catalog.copy(generation = "02")) }
        compose.onNodeWithContentDescription("Reload host resources").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(resources = catalog) }
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Reload host resources").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.CONNECTED,
            pendingWorkspace = WorkspaceCommand.reloadResources(projection, catalog)) }
        compose.onNodeWithContentDescription("Reload host resources").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(pendingWorkspace = null, projection = projection.copy(
            projects = listOf(project.copy(resourcesReloadControl = false)))) }
        compose.onNodeWithContentDescription("Reload host resources").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = listOf(
            project.copy(promptControl = false, resourcesReloadControl = true)))) }
        compose.onNodeWithContentDescription("Reload host resources").assertIsEnabled()
    }
    @Test fun resourcePrependsDraftArgumentsWithoutSendingAndStaleOrOfflineChoicesAreDisabled() {
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", promptControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            resources = ResourceCatalog("project", "session", "2", listOf(
                RemoteResource("research", "skill", "/skill:research", "Find sources", true),
                RemoteResource("review", "template", "/review", "Review change", false)), false, "a".repeat(64))))
        val sends = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onResources = { state.value = state.value.copy(resourcesOpen = true) },
            onPrompt = { sends.add(it) }, onCloseResources = { state.value = state.value.copy(resourcesOpen = false) }) } }
        compose.onNode(hasSetTextAction()).performTextInput("literal $1")
        compose.onNodeWithContentDescription("Select host resource").performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
        compose.onNodeWithText("/review").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithText("/skill:research").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.CONNECTED) }
        val catalog = requireNotNull(state.value.resources)
        val altered = catalog.copy(entries = catalog.entries.map { if (it.invocable) it.copy(command = "!touch marker") else it })
        compose.runOnIdle { state.value = state.value.copy(resources = altered) }
        compose.onNodeWithText("!touch marker").assertIsNotEnabled()
        for (changed in listOf(catalog.copy(projectId = "foreign"), catalog.copy(sessionId = "foreign"))) {
            compose.runOnIdle { state.value = state.value.copy(resources = changed) }
            compose.onNodeWithText("/skill:research").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = state.value.copy(resources = catalog, resourcesError = "changed") }
        compose.onNodeWithText("/skill:research").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(resourcesError = null) }
        compose.onNodeWithText("/skill:research").performClick()
        compose.onNode(hasSetTextAction()).assertTextContains("/skill:research literal $1", substring = true)
        assertTrue(sends.isEmpty()); assertFalse(state.value.resourcesOpen)
    }
    @Test fun pagingControlsNavigateWithoutReplacingDraftOrSending() {
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", promptControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            resources = ResourceCatalog("project", "session", "2", (32 until 64).map {
                RemoteResource("entry-$it", "template", "/entry-$it", "", true) }, true, "a".repeat(64), 32, 64),
            resourcesPrevious = listOf(ResourceCatalog("project", "session", "2", (0 until 32).map {
                RemoteResource("entry-$it", "template", "/entry-$it", "", true) }, true, "a".repeat(64), 0, 32))))
        val offsets = mutableListOf<Int>(); val sends = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onResources = { offsets.add(it) }, onPrompt = { sends.add(it) }) } }
        compose.onNode(hasSetTextAction()).performTextInput("draft arguments")
        compose.runOnIdle { state.value = state.value.copy(resourcesOpen = true) }
        compose.onNodeWithText("/entry-32").assertIsDisplayed()
        compose.onNodeWithText("/entry-63").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Next resource page").assertIsDisplayed().assertIsEnabled().performClick()
        compose.onNodeWithContentDescription("Previous resource page").assertIsDisplayed().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf(64, 0), offsets) }
        val originalCatalog = requireNotNull(state.value.resources)
        val originalPrevious = state.value.resourcesPrevious
        compose.runOnIdle { state.value = state.value.copy(resources = originalCatalog.copy(entries = (64 until 96).map {
            RemoteResource("entry-$it", "template", "/entry-$it", "", true)
        }, offset = 64, nextOffset = 96), resourcesPrevious = originalPrevious + originalCatalog) }
        compose.onNodeWithText("/entry-64").assertIsDisplayed()
        compose.onNodeWithContentDescription("Next resource page").assertIsDisplayed().assertIsEnabled()
        val instrumentation = androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        instrumentation.uiAutomation.waitForIdle(500, 5000)
        val screenshot = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
        java.io.File(instrumentation.targetContext.filesDir, "resources-pinned-pagination.png").outputStream().use {
            assertTrue(screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
        }
        screenshot.recycle()
        compose.runOnIdle { state.value = state.value.copy(resources = originalCatalog, resourcesPrevious = originalPrevious) }
        compose.onNodeWithText("/entry-32").assertIsDisplayed()
        compose.runOnIdle { state.value = state.value.copy(resourcesLoading = true) }
        compose.onNodeWithContentDescription("Next resource page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Previous resource page").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(resourcesOpen = false) }
        compose.onNode(hasSetTextAction()).assertTextContains("draft arguments", substring = true)
        assertTrue(sends.isEmpty())
        val catalog = requireNotNull(state.value.resources)
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(promptControl = false) }), resourcesLoading = false, resourcesOpen = false,
            resources = catalog.copy(entries = listOf(RemoteResource("review", "template", "/review", "Review metadata", true))
                + catalog.entries.drop(1))) }
        compose.onNodeWithContentDescription("Select host resource").assertDoesNotExist()
        compose.onNodeWithContentDescription("Send message").assertDoesNotExist()
        compose.onNodeWithContentDescription("Browse host resources").performScrollTo().assertIsEnabled().performClick()
        compose.runOnIdle { state.value = state.value.copy(resourcesOpen = true) }
        compose.onNodeWithText("/review").assertIsNotEnabled()
        compose.onNodeWithText("Review metadata").assertExists()
        compose.onNodeWithContentDescription("Reload host resources").assertDoesNotExist()
        val ready = state.value
        compose.runOnIdle { state.value = ready.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(promptStatuses = listOf(RemotePromptStatus("running", "project", "session", "running")))
        }), resourcesOpen = false) }
        compose.onNodeWithContentDescription("Select host resource").assertDoesNotExist()
        compose.onNodeWithContentDescription("Browse host resources").performScrollTo().assertIsEnabled().performClick()
        compose.runOnIdle { state.value = state.value.copy(resourcesOpen = true) }
        compose.onNodeWithText("/review").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next resource page").assertIsEnabled()
        val running = state.value
        for (changed in listOf(running.copy(scanInProgress = true), running.copy(resetting = true), running.copy(resetRequired = true))) {
            compose.runOnIdle { state.value = changed }
            compose.onNodeWithContentDescription("Browse host resources").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next resource page").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = running.copy(projection = requireNotNull(running.projection).copy(projects = listOf(
            requireNotNull(running.projection).projects.single(), requireNotNull(running.projection).projects.single().copy(sessionId = "foreign")))) }
        compose.onNodeWithContentDescription("Browse host resources").assertDoesNotExist()
        compose.onNodeWithContentDescription("Next resource page").assertIsNotEnabled()
        compose.runOnIdle { state.value = ready }
        compose.onNodeWithContentDescription("Next resource page").assertIsEnabled().performClick()
        compose.onNodeWithContentDescription("Previous resource page").assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf(64, 0, -1, -1, 64, 0), offsets); assertTrue(sends.isEmpty()) }
        compose.runOnIdle { state.value = state.value.copy(resources = null, resourcesError = "unavailable") }
        compose.onNodeWithContentDescription("Refresh host resources").assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(-1, offsets.last()) }
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Refresh host resources").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Browse host resources").assertIsNotEnabled()
    }
}
