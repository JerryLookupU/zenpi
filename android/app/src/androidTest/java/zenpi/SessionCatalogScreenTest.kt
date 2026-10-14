package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.foundation.layout.Box
import androidx.compose.foundation.layout.width
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.Modifier
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.unit.dp
import org.junit.Assert.assertEquals
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class SessionCatalogScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun conversationHistoryNavigationDisplaysOldMessagesAndFencesOffline() {
        val project = RemoteProject("project", "Workspace", "session")
        val projection = HostProjection(listOf(project), project.id, "session", "idle", emptyList(), "80", true, "10")
        val page = SessionHistoryPage(project.id, "session", "80", "d".repeat(64), 0, 1, 70, true,
            listOf(RemoteTurn("old", "assistant", "Historical conversation preview", true)))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection,
            sessionHistoryOpen = true, sessionHistory = page))
        val pages = mutableListOf<Boolean>(); val turns = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onSessionHistory = { pages += it }, onSessionHistoryTurn = { turns += it },
                onCloseTranscript = { state.value = SessionHistoryReadPolicy.dismissMessage(state.value) })
        } } }
        compose.onNodeWithText("Historical conversation preview").assertExists()
        compose.onNodeWithContentDescription("Previous conversation page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Read conversation message old").performClick()
        compose.onNodeWithContentDescription("Next conversation page").performClick()
        compose.runOnIdle { assertEquals(listOf("old"), turns); assertEquals(listOf(true), pages) }
        val message = TranscriptPage(project.id, "session", "old", "assistant", "a".repeat(64), 0, 13, 13, false, "Full old text", journalRevision = "80")
        compose.runOnIdle { state.value = state.value.copy(transcriptTurn = "old", transcriptHistoryTurn = page.turns.single(), transcriptPage = message) }
        compose.onNodeWithText("Full old text").assertExists()
        compose.onNodeWithText("Historical conversation preview").assertDoesNotExist()
        compose.onNodeWithContentDescription("Close message").performClick()
        compose.onNodeWithText("Historical conversation preview").assertExists()
        compose.onNodeWithText("Full old text").assertDoesNotExist()
        val healthy = state.value
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true),
            healthy.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            healthy.copy(projection = projection.copy(projects = emptyList())), healthy.copy(projection = projection.copy(journalSequence = null)))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Refresh conversation").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next conversation page").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = healthy.copy(sessionHistoryError = "changed") }
        compose.onNodeWithContentDescription("Refresh conversation").assertIsEnabled()
        compose.onNodeWithContentDescription("Next conversation page").assertIsNotEnabled()
        compose.runOnIdle { state.value = healthy }
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Next conversation page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh conversation").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Read conversation message old").assertIsNotEnabled()
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_session_history_ui", "paging_message_return_offline_verified")
        })
    }
    @Test fun resumeRequiresCatalogSelectionConfirmationAndCurrentConnection() {
        val tab = "b".repeat(64)
        val project = RemoteProject("a".repeat(64), "Workspace", "current", sessionControl = true)
        val projection = HostProjection(listOf(project), project.id, "current", "idle", emptyList(), "1", false, "0",
            subtabs = RemoteSubtabs(project.id, "1", tab, listOf(RemoteSubtab(tab, "Main", "main", 1)), false))
        val catalog = SessionCatalog(project.id, "current", "d".repeat(64), 0, 16,
            listOf(RemoteSession("current", "1", 0, true), RemoteSession("history", "1", 3, false,
                matches = 1, snippets = listOf("Mailbox api_key=<redacted>")))
                + (2 until 16).map { RemoteSession("history-$it", "1", 1, false) }, query = "Mailbox")
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, sessionsOpen = true,
            sessions = catalog, sessionsQuery = "Mailbox"))
        val resumed = mutableListOf<String>(); val pages = mutableListOf<Int>(); val searches = mutableListOf<String>()
        compose.setContent { MaterialTheme { Box(Modifier.width(320.dp)) {
            PairingScreen(state.value, {}, {}, {}, onResumeSession = { resumed += it }, onSessions = { pages += it }, onSearchSessions = { searches += it })
        } } }
        compose.onNodeWithContentDescription("Resume session current").assertIsNotEnabled()
        compose.onNodeWithText("1 matches").assertExists()
        compose.onNodeWithText("Mailbox api_key=<redacted>").assertExists()
        compose.onNodeWithTag("session-search").performTextInput("history")
        compose.onNodeWithContentDescription("Search sessions").performClick()
        compose.runOnIdle { assertEquals(listOf("history"), searches) }
        compose.onNodeWithContentDescription("Resume session history").performClick()
        compose.runOnIdle { assertEquals(emptyList<String>(), resumed) }
        compose.onNodeWithContentDescription("Confirm resume session").performClick()
        compose.onNodeWithContentDescription("Next sessions page").performClick()
        compose.runOnIdle { assertEquals(listOf("history"), resumed); assertEquals(listOf(16), pages) }
        compose.onNodeWithContentDescription("Resume session history").performClick()
        compose.onNodeWithContentDescription("Confirm resume session").assertExists()
        compose.runOnIdle { state.value = state.value.copy(sessions = catalog.copy(offset = 16, nextOffset = null,
            entries = listOf(RemoteSession("another", "1", 4, false))), sessionsPrevious = listOf(catalog)) }
        compose.onNodeWithContentDescription("Confirm resume session").assertDoesNotExist()
        compose.onNodeWithContentDescription("Resume session another").performClick()
        compose.onNodeWithContentDescription("Confirm resume session").assertExists()
        compose.runOnIdle { state.value = state.value.copy(sessions = catalog, sessionsPrevious = emptyList()) }
        compose.onNodeWithContentDescription("Confirm resume session").assertDoesNotExist()
        val healthy = state.value
        val tabs = requireNotNull(projection.subtabs)
        compose.runOnIdle { state.value = healthy.copy(sessions = catalog.copy(snapshotId = "invalid")) }
        compose.onNodeWithContentDescription("Resume session history").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next sessions page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Search sessions").assertIsEnabled()
        for (invalid in listOf(tabs.copy(tabs = tabs.tabs + tabs.tabs), tabs.copy(projectId = "foreign"))) {
            compose.runOnIdle { state.value = healthy.copy(projection = projection.copy(subtabs = invalid)) }
            compose.onNodeWithContentDescription("Resume session history").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Search sessions").assertIsEnabled()
        }
        for (blocked in listOf(healthy.copy(scanInProgress = true), healthy.copy(resetting = true), healthy.copy(resetRequired = true),
            healthy.copy(projection = projection.copy(projects = listOf(project, project.copy(sessionId = "foreign")))),
            healthy.copy(projection = projection.copy(projects = emptyList())))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Next sessions page").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Refresh sessions").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Search sessions").assertIsNotEnabled()
            compose.onNodeWithTag("session-search").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = healthy.copy(sessionsError = "changed") }
        compose.onNodeWithContentDescription("Next sessions page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh sessions").assertIsEnabled()
        compose.runOnIdle { state.value = healthy.copy(projection = projection.copy(projects = listOf(project.copy(sessionControl = false)))) }
        compose.onNodeWithContentDescription("Next sessions page").assertIsEnabled()
        compose.onNodeWithContentDescription("Search sessions").assertIsEnabled()
        compose.onNodeWithContentDescription("Resume session history").assertIsNotEnabled()
        compose.runOnIdle { state.value = healthy }
        for (blocked in listOf(state.value.copy(sessions = catalog.copy(projectId = "foreign")),
            state.value.copy(sessions = catalog.copy(sessionId = "other")),
            state.value.copy(projection = projection.copy(projects = listOf(project.copy(sessionId = "other")))),
            state.value.copy(resetRequired = true), state.value.copy(resetting = true), state.value.copy(scanInProgress = true))) {
            compose.runOnIdle { state.value = blocked }
            compose.onNodeWithContentDescription("Resume session history").assertIsNotEnabled()
            compose.onNodeWithContentDescription("Next sessions page").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = state.value.copy(projection = projection, sessions = catalog,
            resetRequired = false, resetting = false, scanInProgress = false) }
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithContentDescription("Resume session history").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next sessions page").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Search sessions").assertIsNotEnabled()
        compose.onNodeWithTag("session-search").assertIsNotEnabled()
        androidx.test.platform.app.InstrumentationRegistry.getInstrumentation().sendStatus(0, android.os.Bundle().apply {
            putString("zenpi_session_search_ui", "body_snippets_confirmation_offline_verified")
        })
    }
}
