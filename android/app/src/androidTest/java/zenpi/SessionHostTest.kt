package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.collectAsState
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.test.platform.app.InstrumentationRegistry
import kotlinx.coroutines.delay
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.lifecycle.LanSocketRouting
import zenpi.pairing.*
import zenpi.security.*
import zenpi.transport.*

class SessionHostTest {
    @get:Rule val compose = createComposeRule()
    @Test fun productionButtonsRenameReplaceAndResumeTheDesktopConversation(): Unit = runBlocking {
        val instrumentation = InstrumentationRegistry.getInstrumentation()
        val args = InstrumentationRegistry.getArguments()
        val namespace = requireNotNull(args.getString("process_namespace"))
        val identity = DeviceIdentity.loadOrCreate("zenpi.session-host.$namespace")
        val store = PairedHostStore(instrumentation.targetContext, namespace)
        val route = LanSocketRouting(instrumentation.targetContext)
        val invite = PairingInvite.parse(requireNotNull(args.getString("invite")))
        val paired = PairingClient(route::bind).pair(invite, invite.safetyCode, "Android-session-test", identity)
        val host = PairedHost(invite.host, invite.port, invite.fingerprint, paired.deviceId)
        store.save(host)
        assertTrue(HostClient(route::bind).inspect(host, identity = identity).projects.isEmpty())
        instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "ungranted_projection_verified") })
        val model = PairingViewModel(instrumentation.targetContext.applicationContext as android.app.Application, store, identity)
        try {
            compose.runOnUiThread { model.setForeground(true) }
            suspend fun awaitProjection(recoverGrant: Boolean = false, predicate: (HostProjection) -> Boolean): HostProjection {
                repeat(150) {
                    val state = model.state.value
                    val cached = store.loadState()?.projection
                    if (state.phase == PairingPhase.CONNECTED && state.pendingWorkspace == null
                        && state.projection?.let { current -> predicate(current) && cached != null
                            && zenpi.state.ProjectionCacheCodec.encode(cached) == zenpi.state.ProjectionCacheCodec.encode(current) } == true) {
                        assertNull(store.loadState()?.pendingWorkspace)
                        return requireNotNull(state.projection)
                    }
                    if (recoverGrant && state.phase == PairingPhase.CONNECTED && state.projection?.projects?.isEmpty() == true)
                        compose.runOnUiThread { model.refresh() }
                    delay(100)
                }
                fail("Session workflow did not converge; phase=${model.state.value.phase}, code=${model.state.value.workspaceFailureCode}")
                error("unreachable")
            }
            val initial = awaitProjection(recoverGrant = true) { it.selectedId == args.getString("expected_project") && it.projects.single().sessionControl }
            val original = requireNotNull(initial.sessionId)
            assertTrue(initial.turns.any { it.content.contains("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT") })
            compose.setContent { MaterialTheme {
                PairingScreen(model.state.collectAsState().value, {}, {}, {}, onRefresh = { model.refresh(it) },
                    onNewSession = model::newSession, onRenameSession = model::renameSession,
                    onSessions = { model.readSessions(it) }, onSearchSessions = model::searchSessions, onCloseSessions = model::closeSessions, onResumeSession = model::resumeSession,
                    onSessionHistory = model::readSessionHistory, onPreviousSessionHistory = model::previousSessionHistory,
                    onCloseSessionHistory = model::closeSessionHistory, onSessionHistoryTurn = { model.readSessionHistoryTurn(it) })
            } }
            compose.onNodeWithContentDescription("Rename session").performScrollTo().performClick()
            compose.onNodeWithText("Session name").performTextReplacement("Android development")
            assertNull(model.state.value.pendingWorkspace)
            compose.onNodeWithContentDescription("Confirm session rename").performClick()
            val renamed = awaitProjection { it.sessionId == original && it.projects.single().sessionName == "Android development" }
            assertEquals(initial.turns, renamed.turns)
            assertEquals(initial.eventSequence, renamed.eventSequence)
            assertEquals(initial.workspaceRevision, renamed.workspaceRevision)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_rename_verified") })
            compose.onNodeWithContentDescription("New session").performScrollTo().performClick()
            assertEquals(original, model.state.value.projection?.sessionId)
            compose.onNodeWithContentDescription("Confirm new session").performClick()
            val replacement = awaitProjection { it.sessionId != original && it.turns.isEmpty() }
            assertEquals(java.math.BigInteger(requireNotNull(initial.workspaceRevision)) + java.math.BigInteger.ONE,
                java.math.BigInteger(requireNotNull(replacement.workspaceRevision)))
            assertEquals("0", replacement.eventSequence)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_new_verified") })
            compose.onNodeWithContentDescription("Browse sessions").performScrollTo().performClick()
            repeat(100) { if (model.state.value.sessions == null && model.state.value.sessionsError == null) delay(100) }
            assertNull(model.state.value.sessionsError)
            val catalog = requireNotNull(model.state.value.sessions)
            assertEquals(2, catalog.entries.size)
            assertEquals("Android development", catalog.entries.single { it.id == original }.name)
            assertTrue(catalog.entries.single { it.id == replacement.sessionId }.current)
            compose.onNodeWithContentDescription("Resume session ${replacement.sessionId}").assertIsNotEnabled()
            compose.onNodeWithTag("session-search").performTextReplacement("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT")
            compose.onNodeWithContentDescription("Search sessions").performClick()
            repeat(100) { if (model.state.value.sessionsLoading) delay(100) }
            assertNull(model.state.value.sessionsError)
            val searched = requireNotNull(model.state.value.sessions)
            assertEquals("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", searched.query)
            assertEquals(catalog.snapshotId, searched.snapshotId)
            assertEquals(listOf(original), searched.entries.map { it.id })
            assertEquals(1, searched.entries.single().matches)
            assertEquals(listOf("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT"), searched.entries.single().snippets)
            assertEquals(replacement, model.state.value.projection)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            compose.onNodeWithText("1 matches").assertExists()
            compose.onNodeWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT").assertExists()
            compose.onNodeWithContentDescription("Resume session $original").performClick()
            assertEquals(replacement.sessionId, model.state.value.projection?.sessionId)
            compose.onNodeWithContentDescription("Confirm resume session").performClick()
            val resumed = awaitProjection { it.sessionId == original && it.turns == initial.turns }
            assertEquals(java.math.BigInteger(requireNotNull(initial.workspaceRevision)) + java.math.BigInteger.valueOf(2),
                java.math.BigInteger(requireNotNull(resumed.workspaceRevision)))
            assertEquals(initial.eventSequence, resumed.eventSequence)
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_session_body_search", "production_host_readonly_resume_verified")
            })
            assertFalse(model.state.value.sessionsOpen)
            compose.onNodeWithContentDescription("Browse conversation").performScrollTo().performClick()
            repeat(100) { if (model.state.value.sessionHistoryLoading) delay(100) }
            assertNull(model.state.value.sessionHistoryError)
            val conversation = requireNotNull(model.state.value.sessionHistory)
            assertEquals(resumed.journalSequence, conversation.journalRevision)
            assertEquals(initial.turns.map { it.id }, conversation.turns.map { it.id })
            assertEquals(resumed, model.state.value.projection)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            compose.onNodeWithContentDescription("Next conversation page").assertIsNotEnabled()
            val historicalTurn = conversation.turns.single { it.content == "STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT" }
            compose.onNodeWithContentDescription("Read conversation message ${historicalTurn.id}").performScrollTo().performClick()
            repeat(100) { if (model.state.value.transcriptLoading) delay(100) }
            assertNull(model.state.value.transcriptError)
            assertEquals(conversation.journalRevision, model.state.value.transcriptPage?.journalRevision)
            assertEquals(historicalTurn.content, model.state.value.transcriptPage?.text)
            assertEquals(resumed, model.state.value.projection)
            compose.onNodeWithContentDescription("Close message").performClick()
            assertEquals(conversation, model.state.value.sessionHistory)
            assertNull(model.state.value.transcriptTurn)
            compose.onNodeWithContentDescription("Close conversation").performClick()
            instrumentation.sendStatus(0, android.os.Bundle().apply {
                putString("zenpi_session_history_host", "production_revision_detail_readonly_verified")
            })
            compose.onNodeWithText("STAGE2_ANDROID_REAL_PROVIDER_TRANSCRIPT", substring = true).performScrollTo().assertIsDisplayed()
            java.io.File(instrumentation.targetContext.filesDir, "session-host.png").outputStream().use {
                assertTrue(instrumentation.uiAutomation.takeScreenshot().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_resume_verified") })
            val cached = requireNotNull(store.loadState()?.projection)
            assertEquals(resumed.turns, cached.turns); assertEquals(original, cached.sessionId)
            assertFalse(cached.projects.single().sessionControl)
            compose.onNodeWithContentDescription("Rename session").performScrollTo().performClick()
            compose.onNodeWithText("Session name").performTextReplacement("Old session draft")
            compose.onNodeWithContentDescription("Confirm session rename").assertIsEnabled()
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_desktop_switch_requested") })
            // No refresh/button call here: only the foreground subscription may
            // observe the desktop transition and acquire a fresh snapshot.
            val followed = awaitProjection { it.sessionId != original && it.sessionId != replacement.sessionId && it.turns.isEmpty() }
            assertEquals(initial.selectedId, followed.selectedId)
            assertEquals(java.math.BigInteger(requireNotNull(resumed.workspaceRevision)) + java.math.BigInteger.ONE,
                java.math.BigInteger(requireNotNull(followed.workspaceRevision)))
            assertEquals("0", followed.eventSequence)
            assertNull(followed.projects.single().sessionName)
            compose.onNodeWithContentDescription("Confirm session rename").assertDoesNotExist()
            compose.onNodeWithText("Old session draft").assertDoesNotExist()
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            val updatedCache = requireNotNull(store.loadState()?.projection)
            assertEquals(followed.sessionId, updatedCache.sessionId); assertTrue(updatedCache.turns.isEmpty())
            assertFalse(updatedCache.projects.single().sessionControl)
            val oldRename = SessionCommandCodec.rename(resumed, "Old captured operation")
            assertFalse(WorkspaceDispatchPolicy.allows(model.state.value, true, true, oldRename))
            assertTrue(WorkspaceDispatchPolicy.allows(model.state.value, true, true,
                SessionCommandCodec.rename(followed, "Current captured operation")))
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            java.io.File(instrumentation.targetContext.filesDir, "session-auto-switch.png").outputStream().use {
                assertTrue(instrumentation.uiAutomation.takeScreenshot().compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
            }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_desktop_switch_verified") })
            compose.runOnUiThread { model.cancel() }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertEquals(host, model.state.value.host); assertTrue(model.state.value.registered)
            assertEquals(followed.sessionId, model.state.value.projection?.sessionId)
            assertFalse(requireNotNull(model.state.value.projection).projects.single().sessionControl)
            compose.runOnUiThread { model.refresh() }
            val reattached = awaitProjection { it.sessionId == followed.sessionId && it.projects.single().sessionControl }
            assertEquals(followed.turns, reattached.turns); assertEquals(followed.eventSequence, reattached.eventSequence)
            compose.runOnUiThread { model.setForeground(false) }
            assertEquals(PairingPhase.OFFLINE, model.state.value.phase)
            assertNull(model.state.value.projection?.workspaceOwner)
            assertFalse(requireNotNull(model.state.value.projection).projects.single().sessionControl)
            compose.runOnUiThread { model.setForeground(true) }
            awaitProjection { it.sessionId == followed.sessionId && it.projects.single().sessionControl }
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_cancel_reattach_verified") })
            compose.runOnUiThread { model.setForeground(false) }
            val beforeLoss = requireNotNull(model.state.value.projection)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_access_loss_requested") })
            var removed = false
            repeat(100) {
                if (!removed) {
                    try { HostClient(route::bind).inspect(host, initial.selectedId, identity = identity) }
                    catch (_: SelectedProjectUnavailableException) { removed = true }
                    catch (error: HostRefusedException) { if (error.code != "unauthorized") throw error }
                    if (!removed) delay(100)
                }
            }
            assertTrue("Host must remove the original project from the authorized listing", removed)
            compose.runOnUiThread { model.setForeground(true) }
            repeat(100) { if (!model.state.value.selectedProjectUnavailable) delay(100) }
            val lost = model.state.value
            assertTrue(lost.selectedProjectUnavailable); assertEquals(PairingPhase.ERROR, lost.phase)
            assertEquals(host, lost.host); assertTrue(lost.registered)
            assertEquals(beforeLoss.turns, lost.projection?.turns); assertEquals(beforeLoss.eventSequence, lost.projection?.eventSequence)
            assertNull(lost.projection?.workspaceOwner)
            assertTrue(requireNotNull(lost.projection).projects.all { !it.sessionControl && !it.promptControl && !it.approvalControl })
            assertNull(lost.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            assertEquals(beforeLoss.sessionId, store.loadState()?.projection?.sessionId)
            assertNotNull(store.loadResumeSnapshot())
            val held = lost
            delay(1200)
            assertEquals(held, model.state.value)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_access_loss_verified") })
            var grantRestored = false
            repeat(100) {
                if (!grantRestored) {
                    try {
                        val probe = HostClient(route::bind).inspect(host, initial.selectedId, identity = identity)
                        grantRestored = probe.sessionId == followed.sessionId && probe.projects.single().sessionControl
                    } catch (_: SelectedProjectUnavailableException) { }
                    catch (error: HostRefusedException) { if (error.code != "unauthorized") throw error }
                    if (!grantRestored) delay(100)
                }
            }
            assertTrue("Host must restore the original project grant before manual refresh", grantRestored)
            assertEquals(held, model.state.value)
            compose.onNodeWithText("Refresh host").performScrollTo().performClick()
            val restored = awaitProjection { it.sessionId == followed.sessionId && it.projects.single().sessionControl }
            assertFalse(model.state.value.selectedProjectUnavailable)
            assertEquals(beforeLoss.turns, restored.turns); assertEquals(beforeLoss.eventSequence, restored.eventSequence)
            assertEquals(followed.workspaceRevision, restored.workspaceRevision)
            assertNull(model.state.value.pendingWorkspace); assertNull(store.loadState()?.pendingWorkspace)
            instrumentation.sendStatus(0, android.os.Bundle().apply { putString("zenpi_phase", "workspace_session_access_restore_verified") })
        } finally {
            compose.runOnUiThread { model.cancel() }
            store.wipe(); identity.wipe()
        }
    }
}
