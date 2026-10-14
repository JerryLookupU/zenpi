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
import zenpi.state.ProjectionCacheCodec

class SubtabScreenTest {
    @get:Rule val compose = createComposeRule()
    private val main = "a".repeat(64)
    private val first = "b".repeat(64)
    private val second = "c".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", true)), "project", "session", "idle",
        emptyList(), "1", false, "0", workspaceOwner = "d".repeat(64), workspaceRevision = "4",
        subtabs = RemoteSubtabs("project", "2", first, listOf(RemoteSubtab(main, "main", "main", 1),
            RemoteSubtab(first, "first", "in_place", 1), RemoteSubtab(second, "second", "worktree", 2)), true))
    @Test fun selectionMoveAndRenameUseStableIdsAndRequireConfirmedName() {
        val actions = mutableListOf<Triple<String, String, String?>>()
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.CONNECTED, projection = projection), {}, {}, {},
            onSubtab = { operation, id, name -> actions.add(Triple(operation, id, name)) }) } }
        compose.onNodeWithTag("secondary-tabs").performScrollTo().performScrollToNode(hasText("second"))
        compose.onNodeWithText("second").performClick()
        compose.runOnIdle { assertEquals(Triple("subtab_select", second, null), actions.single()); actions.clear() }
        compose.onNodeWithContentDescription("Move worktree tab left").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Move worktree tab right").performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(Triple("subtab_move", first, null), actions.single()); actions.clear() }
        compose.onNodeWithContentDescription("Rename worktree tab").performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
        compose.runOnIdle { assertTrue(actions.isEmpty()) }
        compose.onNodeWithText("Tab name").performTextReplacement("new-name")
        compose.onNodeWithText("Tab name").performImeAction()
        compose.onNodeWithText("Rename").performClick()
        compose.runOnIdle { assertEquals(Triple("subtab_rename", first, "new-name"), actions.single()) }
    }
    @Test fun unknownWriteDisablesAllSecondaryActions() {
        val command = WorkspaceCommand.subtab(projection, "subtab_select", second)
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.ERROR, projection = projection,
            pendingWorkspace = command), {}, {}, {}) } }
        compose.onNodeWithText("second").performScrollTo().assertIsNotEnabled()
        compose.onNodeWithContentDescription("Rename worktree tab").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Move worktree tab left").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Move worktree tab right").assertIsNotEnabled()
    }
    @Test fun offlineCacheShowsSecondaryTabsWithoutMutationControls() {
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(projection))
        val actions = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(PairingState(phase = PairingPhase.OFFLINE, projection = cached), {}, {}, {},
            onSubtab = { operation, _, _ -> actions.add(operation) }) } }
        compose.onNodeWithText("first").performScrollTo().assertIsSelected().assertIsNotEnabled()
        compose.onNodeWithText("second").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Rename worktree tab").assertDoesNotExist()
        compose.onNodeWithContentDescription("Move worktree tab right").assertDoesNotExist()
        compose.onNodeWithContentDescription("Add directory tab").assertDoesNotExist()
        assertTrue(actions.isEmpty())
    }
    @Test fun invalidActiveScopeDoesNotCrashAndScannerResetFenceConcurrency() {
        val tabs = requireNotNull(projection.subtabs).copy(directoryControl = true, worktreeControl = true, concurrencyControl = true)
        val healthy = PairingState(phase = PairingPhase.CONNECTED, projection = projection.copy(subtabs = tabs))
        val state = mutableStateOf(healthy)
        val counts = mutableListOf<Pair<String, Int>>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {},
            onConcurrency = { id, count -> counts.add(id to count) }) } }
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithTag("subtab-$second").performScrollTo().assertIsNotEnabled()
            compose.onNodeWithContentDescription("Increase worktree concurrency").performScrollTo().assertIsNotEnabled()
            compose.onNodeWithContentDescription("Rename worktree tab").assertIsNotEnabled()
        }
        for (invalid in listOf(projection.copy(subtabs = tabs.copy(activeId = "f".repeat(64))),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs + tabs.tabs[1])),
            projection.copy(subtabs = tabs.copy(revision = "01")),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs.map { if (it.id == first) it.copy(concurrency = 65) else it })),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs.map { if (it.id == first) it.copy(name = "bad\u001b") else it })),
            projection.copy(subtabs = tabs.copy(tabs = tabs.tabs.map { if (it.id == first) it.copy(kind = "main") else it })),
            projection.copy(subtabs = tabs.copy(projectId = "foreign")), projection.copy(sessionId = "foreign", subtabs = tabs),
            projection.copy(subtabs = tabs, projects = projection.projects + projection.projects.single().copy(sessionId = "foreign")))) {
            compose.runOnIdle { state.value = healthy.copy(projection = invalid) }
            compose.onNodeWithContentDescription("Increase worktree concurrency").assertDoesNotExist()
            compose.onNodeWithContentDescription("Rename worktree tab").assertDoesNotExist()
            compose.onNodeWithTag("subtab-$second").performScrollTo().assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = healthy.copy(projection = projection.copy(subtabs = tabs.copy(tabs = emptyList()))) }
        compose.onNodeWithTag("workspace-tab-project").assertExists()
        for (id in listOf(main, first, second)) compose.onNodeWithTag("subtab-$id").assertDoesNotExist()
        for (control in listOf("Increase worktree concurrency", "Rename worktree tab", "Add directory tab", "Add Git worktree")) {
            compose.onNodeWithContentDescription(control).assertDoesNotExist()
        }
        assertTrue(counts.isEmpty())
        compose.runOnIdle { state.value = healthy }
        compose.onNodeWithContentDescription("Increase worktree concurrency").performScrollTo().performClick()
        compose.runOnIdle { assertEquals(listOf(first to 2), counts) }
    }
}
