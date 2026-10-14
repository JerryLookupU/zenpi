package zenpi.screenshot

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.geometry.Rect
import androidx.compose.ui.geometry.Size
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.semantics.SemanticsNode
import androidx.compose.ui.semantics.SemanticsProperties
import androidx.compose.ui.semantics.getOrNull
import androidx.compose.ui.test.junit4.ComposeContentTestRule
import androidx.compose.ui.test.onRoot
import com.github.takahirom.roborazzi.RoborazziOptions
import com.github.takahirom.roborazzi.captureRoboImage
import org.junit.Assert.assertTrue
import org.junit.rules.TestWatcher
import org.junit.runner.Description
import org.robolectric.RuntimeEnvironment

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

/** The production light scheme from MainActivity; dark uses Material defaults until ZE-535 ships a dark palette. */
@Composable
fun ScreenshotTheme(content: @Composable () -> Unit) {
    MaterialTheme(colorScheme = if (isSystemInDarkTheme()) darkColorScheme() else lightColorScheme(
        primary = Color(0xFF166A4C), secondary = Color(0xFF934A25),
        background = Color(0xFFF5F7F8), surface = Color.White), content = content)
}

/** Goldens are recorded on one OS and verified on another; tolerate anti-aliasing noise only. */
private val options = RoborazziOptions(compareOptions = RoborazziOptions.CompareOptions(changeThreshold = 0.01f))

fun ComposeContentTestRule.captureGolden(name: String, portrait: Portrait) {
    onRoot().captureRoboImage("src/test/resources/golden/${name}_$portrait.png", roborazziOptions = options)
}

/**
 * Fails when any on-screen node straddles the left or right window edge. Content of horizontally
 * scrollable containers (tab rows) is exempt, and nodes wholly off-screen (a closed drawer) are not
 * clipped, they are hidden.
 */
fun ComposeContentTestRule.assertNoHorizontalClipping() {
    val root = onRoot(useUnmergedTree = true).fetchSemanticsNode()
    val width = root.size.width.toFloat()
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
    visit(root)
    assertTrue("horizontally clipped in a ${width}px window: $clipped", clipped.isEmpty())
}
