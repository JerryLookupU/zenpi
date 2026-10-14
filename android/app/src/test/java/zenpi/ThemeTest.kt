package zenpi

import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.ui.test.junit4.createComposeRule
import org.robolectric.RuntimeEnvironment
import org.junit.Assert.assertEquals
import org.junit.Assert.assertNotEquals
import org.junit.Rule
import org.junit.Test
import org.junit.runner.RunWith
import org.robolectric.RobolectricTestRunner
import zenpi.ui.theme.ZenpiDark
import zenpi.ui.theme.ZenpiLight
import zenpi.ui.theme.ZenpiTheme

@RunWith(RobolectricTestRunner::class)
class ThemeTest {
    @get:Rule val compose = createComposeRule()

    private fun scheme(dark: Boolean, dynamic: Boolean): ColorScheme {
        lateinit var scheme: ColorScheme
        compose.setContent { ZenpiTheme(dark = dark, dynamic = dynamic) { scheme = MaterialTheme.colorScheme } }
        compose.waitForIdle()
        return scheme
    }

    @Test fun zenpiPaletteWithoutDynamicColour() {
        assertEquals(ZenpiLight.primary, scheme(dark = false, dynamic = false).primary)
    }

    @Test fun darkPaletteIsItsOwnScheme() {
        val dark = scheme(dark = true, dynamic = false)
        assertEquals(ZenpiDark.surface, dark.surface)
        assertNotEquals(ZenpiLight.surface, dark.surface)
    }

    @Test fun dynamicColourFollowsTheWallpaperScheme() {
        val context = RuntimeEnvironment.getApplication()
        assertEquals(dynamicLightColorScheme(context).primary, scheme(dark = false, dynamic = true).primary)
    }

    @Test fun dynamicDarkColourFollowsTheWallpaperScheme() {
        val context = RuntimeEnvironment.getApplication()
        assertEquals(dynamicDarkColorScheme(context).surface, scheme(dark = true, dynamic = true).surface)
    }
}
