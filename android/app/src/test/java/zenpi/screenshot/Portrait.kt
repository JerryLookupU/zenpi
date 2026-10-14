package zenpi.screenshot

import androidx.compose.runtime.Composable
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.isRoot
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.ExperimentalRoborazziApi
import com.github.takahirom.roborazzi.RoborazziOptions
import com.github.takahirom.roborazzi.captureRoboImage
import com.github.takahirom.roborazzi.captureScreenRoboImage
import org.junit.Assert.assertTrue
import org.junit.rules.TestWatcher
import org.junit.runner.Description
import org.robolectric.RuntimeEnvironment
import zenpi.ui.theme.ZenpiTheme
import androidx.compose.ui.semantics.SemanticsConfiguration

/**
 * One cell of the portrait golden matrix: two phone sizes, font scale 1.0 / 1.3, light / dark.
 * Rendered at mdpi so one golden pixel is one dp and the committed PNGs stay small.
 */
data class Portrait(val width: Int, val height: Int, val fontScale: Float, val dark: Boolean) {
    val qualifiers: String get() = "w${width}dp-h${height}dp-port-" + (if (dark) "night" else "notnight") + "-mdpi"
    override fun toString(): String = "${width}x${height}_font${fontScale}_" + if (dark) "dark" else "light"

    companion object {
        val matrix: List<Portrait> = listOf(360 to 800, 412 to 915).flatMap { (width, height) ->
            listOf(1.0f, 1.3f).flatMap { scale -> listOf(false, true).map { Portrait(width, height, scale, it) } }
        }
    }
}

/** Applies the device configuration before the compose rule launches its activity (outer rule). */
class PortraitRule(private val portrait: Portrait) : TestWatcher() {
    override fun starting(description: Description) {
        RuntimeEnvironment.setQualifiers(portrait.qualifiers)
        RuntimeEnvironment.setFontScale(portrait.fontScale)
    }
}

/** The production theme with Zenpi's own palette: dynamic (wallpaper) colour is off so goldens do not depend on the host. */
@Composable
fun ScreenshotTheme(content: @Composable () -> Unit) = ZenpiTheme(dynamic = false, content = content)

/** Goldens are recorded on one OS and verified on another; tolerate anti-aliasing noise only. */
private val options = RoborazziOptions(compareOptions = RoborazziOptions.CompareOptions(changeThreshold = 0.01f))

fun ComposeContentTestRule.captureGolden(name: String, portrait: Portrait) {
    onRoot().captureRoboImage("src/test/resources/golden/${name}_$portrait.png", roborazziOptions = options)
}

/** Whole-screen golden, including popups such as bottom sheets, which live in their own windows. */
@OptIn(ExperimentalRoborazziApi::class)
fun captureScreenGolden(name: String, portrait: Portrait) {
    captureScreenRoboImage("src/test/resources/golden/${name}_$portrait.png", roborazziOptions = options)
}

/**
 * Fails when any on-screen node straddles the left or right window edge. Content of horizontally
 * scrollable containers (tab rows) is exempt, and nodes wholly off-screen (a closed drawer) are not
 * clipped, they are hidden. Every window is checked (the screen and any popup such as a bottom sheet).
 */
fun ComposeContentTestRule.assertNoHorizontalClipping() {
    val roots = onAllNodes(isRoot(), useUnmergedTree = true).fetchSemanticsNodes()
    val width = roots.maxOf { it.size.width }.toFloat()
    val clipped = mutableListOf<String>()
    fun visit(node: SemanticsNode) {
        if (node.config.getOrNull(SemanticsProperties.HorizontalScrollAxisRange) != null) return
        val bounds = Rect(node.positionInRoot, Size(node.size.width.toFloat(), node.size.height.toFloat()))
        val visible = bounds.right > 0f && bounds.left < width
        if (visible && bounds.width > 0f && (bounds.left < -0.5f || bounds.right > width + 0.5f)) {
            clipped += "${node.config.getOrNull(SemanticsProperties.TestTag) ?: node.config.getOrNull(SemanticsProperties.Text)
                ?: node.config.getOrNull(SemanticsProperties.ContentDescription)} $bounds"
        }
        node.children.forEach(::visit)
    }
    roots.forEach(::visit)
    assertTrue("horizontally clipped in a ${width}px window: $clipped", clipped.isEmpty())
}

/** A UUID or any prefix of one ("4e9b7d0c", "4e9b7d0c-1f3"): how request, session and worker ids are shown. */
private val identifier = Regex("""\b[0-9a-fA-F]{8}(\b|-[0-9a-fA-F])""")

/** Everything TalkBack can read from one node: text, edit text, content and state descriptions. */
private fun SemanticsConfiguration.spoken(): List<String> =
    getOrNull(SemanticsProperties.Text).orEmpty().map { it.text } + getOrNull(SemanticsProperties.ContentDescription).orEmpty() +
        listOfNotNull(getOrNull(SemanticsProperties.EditableText)?.text, getOrNull(SemanticsProperties.StateDescription))

/**
 * Screen-reader contract (ZE-535) over every window: nothing spoken contains a UUID or a UUID prefix
 * (request, session and worker ids are shown at most, never read), and at least one node is a heading.
 */
fun ComposeContentTestRule.assertAccessibleText() {
    val nodes = onAllNodes(isRoot(), useUnmergedTree = true).fetchSemanticsNodes().flatMap { root ->
        generateSequence(listOf(root)) { level -> level.flatMap { it.children }.ifEmpty { null } }.flatten().toList()
    }
    val spoken = nodes.flatMap { it.config.spoken() }.filter { identifier.containsMatchIn(it) }
    assertTrue("TalkBack would read identifiers: $spoken", spoken.isEmpty())
    assertTrue("no heading on screen", nodes.any { it.config.contains(SemanticsProperties.Heading) })
}
