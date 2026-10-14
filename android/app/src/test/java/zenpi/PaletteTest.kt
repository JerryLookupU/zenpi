package zenpi

import androidx.compose.material3.MaterialTheme
import androidx.compose.runtime.getValue
import androidx.compose.runtime.mutableStateOf
import androidx.compose.runtime.setValue
import androidx.compose.ui.semantics.SemanticsActions
import androidx.compose.ui.test.assertIsEnabled
import androidx.compose.ui.test.assertIsNotEnabled
import androidx.compose.ui.test.assertTextContains
import androidx.compose.ui.test.assertTextEquals
import androidx.compose.ui.test.junit4.createComposeRule
import androidx.compose.ui.test.onNodeWithTag
import androidx.compose.ui.test.onNodeWithText
import androidx.compose.ui.test.performScrollTo
import androidx.compose.ui.test.performSemanticsAction
import androidx.compose.ui.test.performTextInput
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNull
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import org.robolectric.annotation.Config
import zenpi.pairing.PairingState
import zenpi.screenshot.Fixtures
import zenpi.transport.RemoteResource
import zenpi.transport.ResourceCatalog
import zenpi.transport.WorkspaceFile
import zenpi.transport.WorkspaceFiles
import zenpi.ui.AppShell
import zenpi.ui.DesktopCommands
import zenpi.ui.PairingActions
import zenpi.ui.PaletteTrigger
import zenpi.ui.PhoneCommand
import java.io.File

@RunWith(RobolectricTestRunner::class)
@Config(qualifiers = "w360dp-h800dp-port")
class PaletteTest {
    @get:Rule val compose = createComposeRule()

    @Test fun triggersOnlyOnATypedSlashOrAtSign() {
        assertEquals(PaletteTrigger.COMMANDS, PaletteTrigger.of("", "/"))
        assertNull(PaletteTrigger.of("a", "a/"))
        assertNull(PaletteTrigger.of("", "/skill"))
        assertEquals(PaletteTrigger.FILES, PaletteTrigger.of("", "@"))
        assertEquals(PaletteTrigger.FILES, PaletteTrigger.of("read ", "read @"))
        assertNull(PaletteTrigger.of("mail", "mail@"))
        assertNull(PaletteTrigger.of("read @x", "read @"))
        assertEquals("", PaletteTrigger.base("/"))
        assertEquals("args", PaletteTrigger.base("/rev args"))
        assertEquals("plain text", PaletteTrigger.base("plain text"))
        assertEquals("read @\"a b.txt\"", PaletteTrigger.attach("read @", "@\"a b.txt\""))
        assertEquals("read @\"a.txt\"", PaletteTrigger.attach("read", "@\"a.txt\""))
        assertEquals("mail@ @\"a.txt\"", PaletteTrigger.attach("mail@", "@\"a.txt\""))
    }

    @Test fun desktopCommandsMirrorTheHostCommandSpecs() {
        var dir: File? = File(System.getProperty("user.dir")).absoluteFile
        while (dir != null && !File(dir, "src/slash.rs").isFile) dir = dir.parentFile
        val source = File(requireNotNull(dir) { "src/slash.rs not found" }, "src/slash.rs").readText()
        val specs = source.substringAfter("pub const COMMAND_SPECS").substringBefore("\n];")
        val names = Regex("""name: "([a-z-]+)"""").findAll(specs).map { it.groupValues[1] }.toList()
        assertEquals(names, DesktopCommands.names)
        assertEquals(PhoneCommand.MODEL, DesktopCommands.phone("reasoning"))
        assertNull(DesktopCommands.phone("yolo"))
    }

    private val resources = ResourceCatalog("zenpi", "s-zenpi", "2", listOf(
        RemoteResource("research", "skill", "/skill:research", "Find sources", true),
        RemoteResource("review", "template", "/review", "Review change", true)), false, "a".repeat(64))
    private val idle = Fixtures.connected.copy(projection = Fixtures.connected.projection!!.copy(phase = "idle", projects =
        Fixtures.connected.projection!!.projects.map { it.copy(promptStatuses = emptyList(), pendingApprovals = emptyList()) }))

    // Robolectric does not route injected touches into a sheet's window; the semantics click runs the same handler.
    private fun tap(text: String) = compose.onNodeWithText(text).performSemanticsAction(SemanticsActions.OnClick)

    @Test fun slashInTheComposerOpensTheCommandPaletteAndAResourceReplacesIt() {
        var state by mutableStateOf(idle)
        val opened = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state, object : PairingActions {
            override fun resources(offset: Int) { opened += "resources:$offset"; state = state.copy(resourcesOpen = true, resources = resources) }
            override fun closeResources() { state = state.copy(resourcesOpen = false) }
            override fun prompt(text: String) { opened += "SENT $text" }
        }) } }
        compose.onNodeWithTag("prompt-composer").performTextInput("/")
        compose.waitForIdle()
        assertEquals(listOf("resources:-1"), opened)
        compose.onNodeWithText("/skill:research").assertIsEnabled()
        // Desktop-only commands are listed and marked, phone-covered ones name their control.
        compose.onNodeWithTag("desktop-command-yolo").performScrollTo().assertIsNotEnabled().assertTextContains("Desktop only", substring = true)
        compose.onNodeWithTag("desktop-command-model").performScrollTo().assertTextContains("Model panel", substring = true)
        tap("/skill:research")
        compose.onNodeWithTag("prompt-composer").assertTextContains("/skill:research ")
        compose.onNodeWithTag("resource-catalog-list").assertDoesNotExist()
        assertEquals(listOf("resources:-1"), opened)
    }

    @Test fun atSignOpensTheFilePickerAndTheChosenFileReplacesIt() {
        val tab = idle.projection!!.subtabs!!
        var state: PairingState by mutableStateOf(idle)
        val paths = mutableListOf<String>()
        compose.setContent { MaterialTheme { AppShell(state, object : PairingActions {
            override fun files(path: String) { paths += path; state = state.copy(filesOpen = true, files = WorkspaceFiles("zenpi", "s-zenpi", tab.activeId,
                tab.revision, ".", listOf(WorkspaceFile("src", "src", true, null, false), WorkspaceFile("notes.md", "notes.md", false, "12", true)), false)) }
            override fun closeFiles() { state = state.copy(filesOpen = false, files = null) }
        }) } }
        compose.onNodeWithTag("prompt-composer").performTextInput("summarize ")
        assertEquals(emptyList<String>(), paths)
        compose.onNodeWithTag("prompt-composer").performTextInput("@")
        compose.waitForIdle()
        assertEquals(listOf("."), paths)
        tap("src")
        assertEquals(listOf(".", "src"), paths)
        tap("notes.md")
        compose.onNodeWithTag("prompt-composer").assertTextEquals("Message", "summarize @\"notes.md\"")
        compose.onNodeWithTag("workspace-files-list").assertDoesNotExist()
    }
}
