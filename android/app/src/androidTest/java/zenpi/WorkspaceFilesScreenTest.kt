package zenpi

import zenpi.ui.PairingScreen
import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.mutableStateOf
import androidx.compose.ui.test.*
import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Assert.*
import org.junit.Rule
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*

class WorkspaceFilesScreenTest {
    @get:Rule val compose = createComposeRule()
    @Test fun filePagingIsReadOnlyAndOfflineOrLoadingCannotNavigate() {
        val tab="b".repeat(64)
        val projection=HostProjection(listOf(RemoteProject("project","Project","session",promptControl=true)),"project","session","idle",emptyList(),"1",false,"0",
            subtabs=RemoteSubtabs("project","1",tab,listOf(RemoteSubtab(tab,"main","main",1)),false))
        val state=mutableStateOf(PairingState(phase=PairingPhase.CONNECTED,projection=projection,filesOpen=true,
            files=WorkspaceFiles("project","session",tab,"1",".",(64 until 128).map {
                WorkspaceFile("file-$it.txt", "file-$it.txt", false, "1", true) },false,WorkspaceFilesPage("d".repeat(64),64,128)),
            filesPrevious=listOf(WorkspaceFiles("project","session",tab,"1",".",(0 until 64).map {
                WorkspaceFile("file-$it.txt", "file-$it.txt", false, "1", true) },false,WorkspaceFilesPage("d".repeat(64),0,64)))))
        val pages=mutableListOf<Int>();val paths=mutableListOf<String>();var sends=0
        compose.setContent{MaterialTheme{PairingScreen(state.value,{},{},{},onFiles={paths.add(it)},onFilePage={pages.add(it)},onPrompt={sends++})}}
        compose.onNodeWithText("file-64.txt").assertIsDisplayed()
        compose.onNodeWithText("file-127.txt").performScrollTo().assertIsDisplayed()
        compose.onNodeWithContentDescription("Previous files page").assertIsDisplayed().assertIsEnabled().performClick()
        compose.onNodeWithContentDescription("Next files page").assertIsDisplayed().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf(0,128),pages);assertEquals(0,sends) }
        val originalFiles = requireNotNull(state.value.files)
        val originalPrevious = state.value.filesPrevious
        compose.runOnIdle { state.value = state.value.copy(files = originalFiles.copy(entries = (128 until 192).map {
            WorkspaceFile("file-$it.txt", "file-$it.txt", false, "1", true)
        }, page = requireNotNull(originalFiles.page).copy(offset = 128, nextOffset = 192)),
            filesPrevious = originalPrevious + originalFiles) }
        compose.onNodeWithText("file-128.txt").assertIsDisplayed()
        compose.onNodeWithContentDescription("Next files page").assertIsDisplayed().assertIsEnabled()
        val instrumentation = androidx.test.platform.app.InstrumentationRegistry.getInstrumentation()
        instrumentation.uiAutomation.waitForIdle(500, 5000)
        val screenshot = requireNotNull(instrumentation.uiAutomation.takeScreenshot())
        java.io.File(instrumentation.targetContext.filesDir, "files-pinned-pagination.png").outputStream().use {
            assertTrue(screenshot.compress(android.graphics.Bitmap.CompressFormat.PNG, 100, it))
        }
        screenshot.recycle()
        compose.runOnIdle { state.value = state.value.copy(files = originalFiles, filesPrevious = originalPrevious) }
        compose.onNodeWithText("file-64.txt").assertIsDisplayed()
        compose.runOnIdle{state.value=state.value.copy(filesLoading=true)}
        compose.onNodeWithContentDescription("Next files page").assertIsNotEnabled()
        compose.runOnIdle{state.value=state.value.copy(filesLoading=false,phase=PairingPhase.OFFLINE)}
        compose.onNodeWithContentDescription("Previous files page").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.CONNECTED, filesOpen = false,
            projection = projection.copy(projects = projection.projects.map { it.copy(promptControl = false) })) }
        compose.onNodeWithContentDescription("Attach host file").assertDoesNotExist()
        compose.onNodeWithContentDescription("Send message").assertDoesNotExist()
        compose.onNodeWithContentDescription("Browse host files").performScrollTo().assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf("."), paths) }
        compose.runOnIdle { state.value = state.value.copy(filesOpen = true, files = requireNotNull(state.value.files).copy(
            entries = listOf(WorkspaceFile("docs", "docs", true, null, false),
                WorkspaceFile("allowed.txt", "allowed.txt", false, "5", true)) + requireNotNull(state.value.files).entries.drop(2))) }
        compose.onNodeWithText("docs").assertIsEnabled().performClick()
        compose.onNodeWithText("allowed.txt").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Next files page").assertIsEnabled().performClick()
        compose.runOnIdle { assertEquals(listOf(".", "docs"), paths); assertEquals(listOf(0, 128, 128), pages); assertEquals(0, sends) }
        compose.runOnIdle { state.value = state.value.copy(files = null, filesError = "unavailable") }
        compose.onNodeWithContentDescription("Refresh host files").assertIsEnabled().performClick()
        assertEquals(listOf(".", "docs", "."), paths)
        compose.runOnIdle { state.value = state.value.copy(scanInProgress = true) }
        compose.onNodeWithContentDescription("Refresh host files").assertIsNotEnabled()
    }
    @Test fun selectedHostFileInsertsQuotedReferenceWithoutSendingAndDirectoriesNavigate() {
        val tab = "b".repeat(64)
        val projection = HostProjection(listOf(RemoteProject("project", "Project", "session", promptControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0",
            subtabs = RemoteSubtabs("project", "1", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), false))
        val state = mutableStateOf(PairingState(phase = PairingPhase.CONNECTED, projection = projection, filesOpen = true,
            files = WorkspaceFiles("project", "session", tab, "1", ".", listOf(
                WorkspaceFile("docs", "docs", true, null, false), WorkspaceFile("file name.txt", "file name.txt", false, "5", true),
                WorkspaceFile("large.bin", "large.bin", false, "10485761", false)), false)))
        val paths = mutableListOf<String>(); val sends = mutableListOf<String>()
        compose.setContent { MaterialTheme { PairingScreen(state.value, {}, {}, {}, onFiles = { paths.add(it) },
            onPrompt = { sends.add(it) }, onCloseFiles = { state.value = state.value.copy(filesOpen = false, files = null) }) } }
        compose.onNodeWithText("large.bin").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.OFFLINE) }
        compose.onNodeWithText("docs").assertIsNotEnabled(); compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(phase = PairingPhase.CONNECTED) }
        val files = requireNotNull(state.value.files)
        compose.runOnIdle { state.value = state.value.copy(files = files.copy(entries = files.entries.map {
            if (it.attachable) it.copy(sizeBytes = "10485761") else it })) }
        compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        compose.onNodeWithText("docs").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(files = files) }
        val tabs = requireNotNull(projection.subtabs)
        val inactive = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(
            subtabs = tabs.copy(tabs = tabs.tabs + listOf(inactive, inactive)))) }
        compose.onNodeWithText("docs").assertIsNotEnabled()
        compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        compose.onNodeWithContentDescription("Refresh host files").assertDoesNotExist()
        compose.runOnIdle { state.value = state.value.copy(projection = projection) }
        for (changed in listOf(files.copy(sessionId = "new"), files.copy(projectId = "foreign"), files.copy(subtabRevision = "2"))) {
            compose.runOnIdle { state.value = state.value.copy(files = changed) }
            compose.onNodeWithText("docs").assertIsNotEnabled(); compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        }
        compose.runOnIdle { state.value = state.value.copy(files = files, filesError = "changed") }
        compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(filesError = null) }
        compose.runOnIdle { state.value = state.value.copy(projection = projection.copy(projects = projection.projects.map {
            it.copy(promptStatuses = listOf(RemotePromptStatus("request", "project", "session", "running")))
        })) }
        compose.onNodeWithText("docs").assertIsEnabled()
        compose.onNodeWithText("file name.txt").assertIsNotEnabled()
        compose.runOnIdle { state.value = state.value.copy(filesOpen = false) }
        compose.onNodeWithContentDescription("Attach host file").assertDoesNotExist()
        compose.onNodeWithContentDescription("Browse host files").assertIsEnabled()
        compose.runOnIdle { state.value = state.value.copy(projection = projection, filesOpen = true) }
        compose.onNodeWithText("docs").performClick(); assertEquals(listOf("docs"), paths)
        compose.onNodeWithText("file name.txt").performClick()
        compose.onNode(hasSetTextAction()).assertTextContains("@\"file name.txt\"", substring = true)
        assertTrue(sends.isEmpty()); assertFalse(state.value.filesOpen)
    }
}
