package zenpi

import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*
import zenpi.state.ProjectionCacheCodec

class TabCapacityScreenTest {
    @get:Rule val compose = createComposeRule()
    private val projects = (0 until 64).map { RemoteProject("project-$it", "Workspace-$it-" + "long-name".repeat(8), "session-$it", true) }
    private val tabs = (0 until 32).map { RemoteSubtab(it.toString(16).padStart(64, '0'),
        if (it == 0) "main" else "Directory-$it-" + "long-name".repeat(8), if (it == 0) "main" else "in_place", 1) }
    private val projection = HostProjection(projects, projects.last().id, projects.last().sessionId, "idle", emptyList(), "1", false, "0",
        workspaceOwner = "a".repeat(64), workspaceRevision = "1",
        subtabs = RemoteSubtabs(projects.last().id, "1", tabs.last().id, tabs, true, true))
    private fun screenshot(name: String) {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val bitmap = instrumentation.uiAutomation.takeScreenshot()
        java.io.File(instrumentation.targetContext.filesDir, name).outputStream().use {
            bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
        }
        bitmap.recycle()
    }
    @Test fun maximumTabsHaveReadableTargetsAndRouteStableIdsOnNarrowScreen() {
        val workspaceActions = mutableListOf<String>(); val secondaryActions = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection), {}, {}, {},
                onWorkspace = { _, id, _ -> workspaceActions.add(id) }, onSubtab = { _, id, _ -> secondaryActions.add(id) })
        } } }
        compose.onNodeWithTag("workspace-tabs").performScrollTo()
            .performScrollToNode(hasTestTag("workspace-tab-${projects.last().id}"))
        compose.onNodeWithTag("workspace-tab-${projects.last().id}").assertIsSelected().assertIsDisplayed()
        val primary = compose.onNodeWithTag("workspace-tab-${projects.last().id}").getUnclippedBoundsInRoot()
        assertTrue((primary.right - primary.left).value >= 96); assertTrue((primary.right - primary.left).value <= 220); assertTrue((primary.bottom - primary.top).value >= 48)
        compose.onNodeWithTag("secondary-tabs").performScrollTo()
            .performScrollToNode(hasTestTag("subtab-${tabs.last().id}"))
        compose.onNodeWithTag("subtab-${tabs.last().id}").assertIsSelected().assertIsDisplayed()
        val secondary = compose.onNodeWithTag("subtab-${tabs.last().id}").getUnclippedBoundsInRoot()
        assertTrue((secondary.right - secondary.left).value >= 96); assertTrue((secondary.right - secondary.left).value <= 220); assertTrue((secondary.bottom - secondary.top).value >= 48)
        screenshot("tab-capacity-online.png")
        compose.onNodeWithTag("secondary-tabs").performScrollToNode(hasTestTag("subtab-${tabs[1].id}"))
        compose.onNodeWithTag("subtab-${tabs[1].id}").performClick()
        assertEquals(listOf(tabs[1].id), secondaryActions)
        compose.onNodeWithTag("workspace-tabs").performScrollToNode(hasTestTag("workspace-tab-${projects[1].id}"))
        compose.onNodeWithTag("workspace-tab-${projects[1].id}").performClick()
        assertEquals(listOf(projects[1].id), workspaceActions)
    }
    @Test fun maximumCachedTabsRemainScrollableWithoutMutationAuthority() {
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection))
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(PairingState(phase = PairingPhase.OFFLINE, projection = cached), {}, {}, {})
        } } }
        compose.onNodeWithTag("secondary-tabs").performScrollTo()
            .performScrollToNode(hasTestTag("subtab-${tabs.last().id}"))
        compose.onNodeWithTag("subtab-${tabs.last().id}").assertIsSelected().assertIsNotEnabled().assertIsDisplayed()
        screenshot("tab-capacity-offline.png")
        compose.onNodeWithTag("secondary-tabs").performScrollToNode(hasTestTag("subtab-${tabs[1].id}"))
        compose.onNodeWithTag("subtab-${tabs[1].id}").assertIsNotEnabled().assertIsDisplayed()
        compose.onNodeWithContentDescription("Rename worktree tab").assertDoesNotExist()
        assertNull(cached.workspaceOwner); assertFalse(requireNotNull(cached.subtabs).control)
    }
}
