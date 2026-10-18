package zenpi.screenshot

import androidx.compose.ui.test.junit4.createComposeRule
import org.junit.Rule
import org.junit.Test
import org.junit.rules.RuleChain
import org.junit.runner.RunWith
import org.robolectric.ParameterizedRobolectricTestRunner
import org.robolectric.annotation.GraphicsMode
import zenpi.pairing.PairingState
import zenpi.transport.RemoteResource
import zenpi.transport.ResourceCatalog
import zenpi.transport.WorkspaceFile
import zenpi.transport.WorkspaceFiles
import zenpi.ui.AppShell
import zenpi.ui.PairingActions

/** The composer's `/` command palette and `@` file picker, in every portrait cell. */
@RunWith(ParameterizedRobolectricTestRunner::class)
@GraphicsMode(GraphicsMode.Mode.NATIVE)
class PaletteScreenshotTest(private val portrait: Portrait) {
    private val compose = createComposeRule()
    @get:Rule val rules: RuleChain = RuleChain.outerRule(PortraitRule(portrait)).around(compose)

    private val idle = Fixtures.connected.copy(projection = Fixtures.connected.projection!!.copy(phase = "idle", projects =
        Fixtures.connected.projection!!.projects.map { it.copy(promptStatuses = emptyList(), pendingApprovals = emptyList()) }))

    private fun capture(state: PairingState, name: String) {
        compose.setContent { ScreenshotTheme { AppShell(state, object : PairingActions {}) } }
        compose.waitForIdle()
        compose.assertNoHorizontalClipping()
        compose.assertAccessibleText()
        captureScreenGolden(name, portrait)
    }

    @Test fun commandPalette() = capture(idle.copy(resourcesOpen = true, resources = ResourceCatalog("zenpi", "s-zenpi", "2", listOf(
        RemoteResource("research", "skill", "/skill:research", "Find and cite sources", true),
        RemoteResource("review", "template", "/review", "Review the pending change", true)), false, "a".repeat(64))), "palette_commands")

    @Test fun filePalette() = idle.projection!!.subtabs!!.let { tabs ->
        capture(idle.copy(filesOpen = true, files = WorkspaceFiles("zenpi", "s-zenpi", tabs.activeId, tabs.revision, ".", listOf(
            WorkspaceFile("src", "src", true, null, false), WorkspaceFile("Cargo.toml", "Cargo.toml", false, "2048", true),
            WorkspaceFile("README.md", "README.md", false, "4096", true)), false)), "palette_files")
    }

    companion object {
        @JvmStatic
        @ParameterizedRobolectricTestRunner.Parameters(name = "{0}")
        fun portraits(): List<Array<Any>> = Portrait.matrix.map { arrayOf(it) }
    }
}
