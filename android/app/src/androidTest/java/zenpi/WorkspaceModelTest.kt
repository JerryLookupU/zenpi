package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.runtime.getValue
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.*
import zenpi.lifecycle.LanSocketRouting

class WorkspaceModelTest {
    private fun hostClient() = zenpi.transport.HostClient(LanSocketRouting(InstrumentationRegistry.getInstrumentation().targetContext)::bind)
    @get:Rule val compose = createComposeRule()
    @Test fun productionViewModelButtonsMutateTheSharedDesktopOwner() = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        val child = requireNotNull(args.getString("workspace_child"))
        val initial = requireNotNull(args.getString("expected_project"))
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        val identity = DeviceIdentity.loadOrCreate("zenpi.network-process.$namespace")
        val model = PairingViewModel(instrumentation.targetContext.applicationContext as android.app.Application, store, identity)
        val subtabClicks = mutableListOf<String>()
        try {
            repeat(30) { if (model.state.value.host == null) delay(100) }
            compose.runOnUiThread { model.setForeground(true) }
            repeat(50) { if (model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            compose.setContent { MaterialTheme {
                val state by model.state.collectAsState()
                PairingScreen(state, {}, model::confirm, model::cancel, onForget = model::forget,
                    onRefresh = { model.refresh(it) }, onWorkspace = model::workspace, onRetryWorkspace = model::retryWorkspace, onSubtab = { operation, id, text -> subtabClicks.add(id); model.subtab(operation, id, text) }, onAddDirectoryTab = model::addDirectoryTab,
                    onPrompt = model::prompt, onSteer = model::steer, onCancelPrompt = model::cancelPrompt, onAddWorktreeTab = model::addWorktreeTab,
                    onDiff = model::reviewDiff, onCloseDiff = model::closeDiff,
                    onNextDiff = model::nextDiff, onPreviousDiff = model::previousDiff)
            } }
            val stableMain = requireNotNull(model.state.value.projection?.subtabs).activeId
            compose.onNodeWithContentDescription("Rename worktree tab").performScrollTo().performClick()
            compose.onNodeWithText("Tab name").performTextReplacement("ui-main")
            compose.onNodeWithText("Tab name").performImeAction(); compose.waitForIdle()
            compose.onNodeWithText("Rename").performClick()
            repeat(60) { if (model.state.value.projection?.subtabs?.tabs?.single()?.name != "ui-main" || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals("ui-main", model.state.value.projection?.subtabs?.tabs?.single()?.name)
            assertEquals(stableMain, model.state.value.projection?.subtabs?.activeId)
            assertNull(store.loadState()?.pendingWorkspace)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "production_subtab_buttons_verified") })
            suspend fun awaitTabs(count: Int, activeName: String) {
                val original = model.state.value.pendingWorkspace?.envelope?.deepCopy()
                var retries = 0
                repeat(60) {
                    val state = model.state.value
                    val tabs = state.projection?.subtabs
                    val pending = state.pendingWorkspace
                    if (state.phase == PairingPhase.ERROR && pending != null && retries < 3) {
                        assertEquals(original, pending.envelope)
                        compose.onNodeWithText("Retry same request").performClick()
                        retries++
                    }
                    if (state.phase != PairingPhase.CONNECTED || tabs?.tabs?.size != count
                        || tabs.tabs.find { it.id == tabs.activeId }?.name != activeName) delay(100)
                }
                val tabs = requireNotNull(model.state.value.projection?.subtabs)
                assertEquals("${model.state.value.error}; code=${model.state.value.workspaceFailureCode}; expected=$activeName", PairingPhase.CONNECTED, model.state.value.phase)
                assertEquals(count, tabs.tabs.size); assertEquals(activeName, tabs.tabs.single { it.id == tabs.activeId }.name)
                assertNull(store.loadState()?.pendingWorkspace)
            }
            for ((index, name) in listOf("directory-one", "directory-two").withIndex()) {
                compose.onNodeWithContentDescription("Add directory tab").performScrollTo().performClick()
                compose.onNodeWithText("Directory tab name").performTextReplacement(name)
                compose.onNodeWithText("Project subdirectory").performTextReplacement(name)
                compose.onNodeWithText("Project subdirectory").performImeAction(); compose.waitForIdle()
                compose.onNodeWithText("Add").performClick()
                awaitTabs(index + 2, name)
            }
            val tabs = requireNotNull(model.state.value.projection?.subtabs)
            val firstId = tabs.tabs.single { it.name == "directory-one" }.id
            val secondId = tabs.tabs.single { it.name == "directory-two" }.id
            compose.onNodeWithText("Desktop").performScrollTo()
            compose.onNodeWithContentDescription("Move worktree tab left").performScrollTo().performClick()
            repeat(60) { if (model.state.value.projection?.subtabs?.tabs?.get(1)?.id != secondId || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(secondId, model.state.value.projection?.subtabs?.tabs?.get(1)?.id)
            assertEquals(secondId, model.state.value.projection?.subtabs?.activeId)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_order_verified") })
            delay(400)
            val liveTabs = requireNotNull(model.state.value.projection?.subtabs)
            val offline = PairingViewModel(instrumentation.targetContext.applicationContext as android.app.Application,
                PairedHostStore(instrumentation.targetContext, namespace), identity)
            try {
                repeat(30) { if (offline.state.value.projection == null) delay(100) }
                assertEquals(PairingPhase.OFFLINE, offline.state.value.phase)
                assertEquals(liveTabs.copy(control = false, directoryControl = false, worktreeControl = false,
                    concurrencyControl = false), offline.state.value.projection?.subtabs)
                assertFalse(requireNotNull(offline.state.value.projection).projects.any { it.workspaceControl })
                compose.runOnUiThread { offline.subtab("subtab_select", firstId) }
                assertNull(offline.state.value.pendingWorkspace)
                assertNull(store.loadState()?.pendingWorkspace)
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_offline_cache_verified") })
            } finally { compose.runOnUiThread { offline.cancel() } }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_selection_stress_start") })
            repeat(12) { index ->
                val name = if (index % 2 == 0) "directory-one" else "directory-two"
                compose.onNodeWithTag("secondary-tabs").performScrollTo().assertIsDisplayed()
                compose.onNodeWithText(name).performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
                awaitTabs(3, name)
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_selection_stress_verified") })
            delay(400)
            compose.onNodeWithTag("secondary-tabs").performScrollTo().assertIsDisplayed()
            compose.onNodeWithText("directory-one").performScrollTo().assertIsDisplayed().assertIsEnabled().performClick(); awaitTabs(3, "directory-one")
            assertEquals(firstId, model.state.value.projection?.subtabs?.activeId)
            for (name in listOf("directory-one", "directory-two")) {
                if (name == "directory-two") {
                    compose.onNodeWithTag("secondary-tabs").performScrollTo().assertIsDisplayed()
                    val button = compose.onNodeWithText(name)
                    button.performScrollTo()
                    compose.waitUntil(5_000) { !button.fetchSemanticsNode().config.contains(androidx.compose.ui.semantics.SemanticsProperties.Disabled) }
                    val clicksBefore = subtabClicks.size
                    assertTrue(runCatching { zenpi.transport.WorkspaceCommand.subtab(requireNotNull(model.state.value.projection), "subtab_select", secondId) }.isSuccess)
                    val bitmap = instrumentation.uiAutomation.takeScreenshot()
                    java.io.File(instrumentation.targetContext.filesDir, "directory-second-before-click.png").outputStream().use {
                        bitmap.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it)
                    }
                    bitmap.recycle()
                    button.performScrollTo().assertIsDisplayed().assertIsEnabled().performClick()
                    compose.runOnIdle {
                        assertEquals("Second directory tab callback missing", clicksBefore + 1, subtabClicks.size)
                        assertEquals(secondId, subtabClicks.last())
                    }
                    awaitTabs(3, name)
                }
                val marker = "STAGE2_ANDROID_DIRECTORY_${if (name == "directory-one") "ONE" else "TWO"}"
                compose.onNodeWithText("Message").performScrollTo().performTextReplacement(marker)
                compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
                repeat(120) {
                    if (model.state.value.phase != PairingPhase.CONNECTED || model.state.value.projection?.turns?.none { it.content.contains("${marker}_DONE") } != false) {
                        delay(100)
                        if (model.state.value.phase == PairingPhase.ERROR && model.state.value.pendingWorkspace != null)
                            compose.runOnUiThread { model.retryWorkspace() }
                    }
                }
                assertTrue(requireNotNull(model.state.value.projection).turns.any { it.content.contains("${marker}_DONE") })
                assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
                assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
                assertEquals(if (name == "directory-one") firstId else secondId, model.state.value.projection?.subtabs?.activeId)
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_execution_verified") })
            compose.onNodeWithTag("secondary-tabs").performScrollTo().assertIsDisplayed()
            compose.onNodeWithText("directory-one").performScrollTo().assertIsDisplayed().assertIsEnabled().performClick(); awaitTabs(3, "directory-one")
            compose.onNodeWithContentDescription("Close directory tab").performScrollTo().performClick()
            compose.onNodeWithText("Close").performClick(); awaitTabs(2, "directory-two")
            compose.onNodeWithContentDescription("Close directory tab").performScrollTo().performClick()
            compose.onNodeWithText("Close").performClick(); awaitTabs(1, "ui-main")
            assertEquals(stableMain, model.state.value.projection?.subtabs?.activeId)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "directory_tabs_verified") })
            if (args.getString("worktree_workflow") == "true") {
                compose.onNodeWithText("Desktop").performScrollTo()
                compose.onNodeWithContentDescription("Add Git worktree").performScrollTo().performClick()
                compose.onNodeWithText("New branch name").performTextReplacement("android-feature")
                compose.onNodeWithText("New branch name").performImeAction(); compose.waitForIdle()
                compose.onNodeWithText("Create").performClick(); awaitTabs(2, "android-feature")
                val worktreeId = requireNotNull(model.state.value.projection?.subtabs).activeId
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_created_verified") })
                compose.onNodeWithText("Message").performScrollTo().performTextReplacement("STAGE2_ANDROID_WORKTREE_READ")
                compose.onNodeWithContentDescription("Send message").performScrollTo().performClick()
                repeat(120) {
                    if (model.state.value.phase != PairingPhase.CONNECTED || model.state.value.projection?.turns?.none { it.content.contains("STAGE2_ANDROID_WORKTREE_READ_DONE") } != false) delay(100)
                }
                assertTrue(requireNotNull(model.state.value.projection).turns.any { it.content.contains("STAGE2_ANDROID_WORKTREE_READ_DONE") })
                assertEquals(worktreeId, model.state.value.projection?.subtabs?.activeId)
                assertNull(store.loadState()?.pendingWorkspace)
                compose.onNodeWithContentDescription("Review diff").performScrollTo().performClick()
                repeat(60) { if (model.state.value.diff == null && model.state.value.diffError == null) delay(100) }
                assertNull(model.state.value.diffError)
                assertEquals(worktreeId, requireNotNull(model.state.value.diff).subtabId)
                assertTrue(requireNotNull(model.state.value.diff).diff.contains("WORKTREE_ONLY_CONTENT"))
                assertTrue(requireNotNull(model.state.value.diff).diff.contains("-TRACKED_WORKTREE_CONTENT"))
                assertTrue(requireNotNull(model.state.value.diff).diff.contains("+WORKTREE_ONLY_CONTENT"))
                compose.onNodeWithTag("diff-patch").assertTextContains("+WORKTREE_ONLY_CONTENT", substring = true).assertIsDisplayed()
                compose.onNodeWithContentDescription("Close diff").performClick()
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_diff_ui_verified") })
                suspend fun awaitHostDiff(matches: (zenpi.transport.RemoteDiff) -> Boolean) = kotlinx.coroutines.withTimeout(10_000) {
                    while (true) {
                        val host = requireNotNull(model.state.value.host)
                        val projection = requireNotNull(model.state.value.projection)
                        val value = runCatching { hostClient().diff(host, identity, projection, "diff-pages.txt") }.getOrNull()
                        if (value != null && matches(value)) break
                        delay(100)
                    }
                }
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_diff_pages_requested") })
                awaitHostDiff { it.page?.totalBytes == requireNotNull(args.getString("diff_total_bytes")).toInt() }
                compose.runOnUiThread { model.reviewDiff("diff-pages.txt") }
                suspend fun awaitDiff(offset: Int) {
                    compose.waitUntil(10_000) {
                        val state = model.state.value
                        !state.diffLoading && (state.diffError != null
                            || state.diff?.page?.offset == offset && DiffReadPolicy.navigation(state))
                    }
                    assertNull(model.state.value.diffError)
                    assertEquals(worktreeId, requireNotNull(model.state.value.diff).subtabId)
                    assertEquals(offset, requireNotNull(model.state.value.diff?.page).offset)
                    compose.waitForIdle()
                }
                awaitDiff(0)
                val firstPage = requireNotNull(model.state.value.diff)
                val firstMetadata = requireNotNull(firstPage.page)
                assertFalse(firstPage.truncated)
                assertTrue(firstMetadata.totalBytes > 65536)
                val patch = StringBuilder(firstPage.diff)
                var pageCount = 1
                while (requireNotNull(model.state.value.diff?.page).hasMore) {
                    val prior = requireNotNull(model.state.value.diff)
                    assertTrue("phase=${model.state.value.phase}; error=${model.state.value.diffError}; page=${prior.page}; history=${model.state.value.diffPrevious.map { it.page }}",
                        DiffReadPolicy.navigation(model.state.value))
                    compose.onNodeWithContentDescription("Next diff page").assertIsDisplayed().assertIsEnabled().performClick()
                    awaitDiff(requireNotNull(prior.page).nextOffset)
                    val next = requireNotNull(model.state.value.diff)
                    assertEquals(firstMetadata.digest, requireNotNull(next.page).digest)
                    if (pageCount == 1) {
                        compose.onNodeWithContentDescription("Previous diff page").assertIsEnabled().performClick()
                        compose.waitUntil(5_000) { model.state.value.diff == firstPage }
                        compose.onNodeWithContentDescription("Next diff page").performClick()
                        awaitDiff(requireNotNull(prior.page).nextOffset)
                        assertEquals(next, model.state.value.diff)
                    }
                    patch.append(next.diff)
                    pageCount++
                    assertTrue("Unbounded diff continuation", pageCount < 32)
                }
                assertTrue(pageCount > 8)
                assertEquals(requireNotNull(args.getString("diff_total_bytes")).toInt(), patch.toString().toByteArray(Charsets.UTF_8).size)
                val digest = java.security.MessageDigest.getInstance("SHA-256").digest(patch.toString().toByteArray(Charsets.UTF_8))
                    .joinToString("") { "%02x".format(it.toInt() and 255) }
                assertEquals(args.getString("diff_patch_sha256"), digest)
                compose.onNodeWithContentDescription("Next diff page").assertIsNotEnabled()
                instrumentation.sendStatus(0, android.os.Bundle().apply {
                    putString("zenpi_phase", "worktree_diff_pages_verified")
                    putString("zenpi_diff_patch_sha256", digest)
                    putString("zenpi_diff_total_bytes", patch.toString().toByteArray(Charsets.UTF_8).size.toString())
                })
                awaitHostDiff { it.page?.digest != firstMetadata.digest }
                compose.onNodeWithContentDescription("Previous diff page").performClick()
                val retained = requireNotNull(model.state.value.diff)
                compose.onNodeWithContentDescription("Next diff page").performClick()
                compose.waitUntil(10_000) { !model.state.value.diffLoading && model.state.value.diffError != null }
                assertEquals(retained, model.state.value.diff)
                compose.onNodeWithContentDescription("Next diff page").assertIsNotEnabled()
                compose.onNodeWithContentDescription("Refresh diff").performClick()
                awaitDiff(0)
                assertNotEquals(firstMetadata.digest, model.state.value.diff?.page?.digest)
                compose.onNodeWithContentDescription("Close diff").performClick()
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_diff_changed_verified") })
                awaitHostDiff { !it.changed && it.page?.totalBytes == 0 }
                compose.onNodeWithText("Desktop").performScrollTo()
                compose.onNodeWithContentDescription("Remove Git worktree").performScrollTo().performClick()
                compose.onNodeWithText("Remove").performClick()
                repeat(60) { if (model.state.value.phase != PairingPhase.ERROR) delay(100) }
                assertEquals("Host request refused: invalid_request", model.state.value.error)
                assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
                assertEquals(worktreeId, model.state.value.projection?.subtabs?.activeId)
                assertEquals(2, model.state.value.projection?.subtabs?.tabs?.size)
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_dirty_refusal_verified") })
                compose.runOnUiThread { model.refresh() }; awaitTabs(2, "android-feature")
                compose.onNodeWithText("Desktop").performScrollTo()
                compose.onNodeWithContentDescription("Remove Git worktree").performScrollTo().performClick()
                compose.onNodeWithText("Remove").performClick(); awaitTabs(1, "ui-main")
                assertEquals(stableMain, model.state.value.projection?.subtabs?.activeId)
                instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "worktree_removed_verified") })
            }
            compose.onNodeWithContentDescription("Open workspace").performScrollTo().performClick()
            compose.onNodeWithText("Host directory").performTextInput("child")
            compose.onNodeWithText("Host directory").performImeAction(); compose.waitForIdle()
            compose.onNodeWithText("Open").performClick()
            repeat(60) { if (model.state.value.projection?.selectedId != child || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(child, model.state.value.projection?.selectedId)
            assertEquals(PairingPhase.CONNECTED, model.state.value.phase)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_open_verified") })
            delay(400)
            compose.onNodeWithContentDescription("Close workspace").performScrollTo().performClick()
            compose.onNodeWithText("Close").performClick()
            repeat(60) { if (model.state.value.projection?.selectedId != initial || model.state.value.phase != PairingPhase.CONNECTED) delay(100) }
            assertEquals(initial, model.state.value.projection?.selectedId)
            assertEquals(1, model.state.value.projection?.projects?.size)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_close_verified") })
            compose.onNodeWithContentDescription("Open workspace").performScrollTo().performClick()
            compose.onNodeWithText("Host directory").performTextReplacement("missing-directory")
            compose.onNodeWithText("Host directory").performImeAction(); compose.waitForIdle()
            compose.onNodeWithText("Open").performClick()
            repeat(60) { if (model.state.value.phase != PairingPhase.ERROR) delay(100) }
            // A transport/auth read can be busy before the write response is received.
            // Resolve only the original pending ID; never create a replacement command.
            val originalPending = model.state.value.pendingWorkspace?.envelope
            repeat(3) {
                if (model.state.value.pendingWorkspace != null) {
                    assertEquals(originalPending, model.state.value.pendingWorkspace?.envelope)
                    compose.runOnUiThread { model.retryWorkspace() }
                    repeat(60) { if (model.state.value.phase == PairingPhase.CONNECTING) delay(100) }
                }
            }
            assertEquals(PairingPhase.ERROR, model.state.value.phase)
            assertEquals("Host request refused: invalid_request", model.state.value.error)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            assertEquals(initial, model.state.value.projection?.selectedId)
            compose.runOnUiThread { model.setForeground(false) }
            val identityBefore = requireNotNull(store.loadState())
            val current = hostClient().inspect(identityBefore.host, identity = identity)
            val confirmed = zenpi.transport.WorkspaceCommand.subtab(current, "subtab_rename", stableMain, "ui-main")
            store.beginWorkspace(identityBefore.host, confirmed)
            var confirmedCallback = false
            try {
                hostClient().inspect(identityBefore.host, identity = identity, workspaceCommand = confirmed,
                    onWorkspaceConfirmed = {
                        store.finishWorkspace(identityBefore.host, confirmed)
                        confirmedCallback = true
                        throw zenpi.transport.WireException("injected post-receipt read failure")
                    })
                fail("Injected post-receipt failure was not observed")
            } catch (error: zenpi.transport.WireException) {
                assertEquals("injected post-receipt read failure", error.message)
            }
            assertTrue(confirmedCallback)
            assertNull(store.loadState()?.pendingWorkspace)
            val recovered = hostClient().inspect(identityBefore.host, identity = identity)
            assertEquals("ui-main", recovered.subtabs?.tabs?.single()?.name)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "confirmed_write_read_failure_verified") })
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "production_workspace_buttons_verified") })
        } finally { compose.runOnUiThread { model.setForeground(false); model.cancel() } }
    }
}
