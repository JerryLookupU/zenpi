package zenpi.ui.theme

import androidx.compose.foundation.isSystemInDarkTheme
import androidx.compose.material3.ColorScheme
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.darkColorScheme
import androidx.compose.material3.dynamicDarkColorScheme
import androidx.compose.material3.dynamicLightColorScheme
import androidx.compose.material3.lightColorScheme
import androidx.compose.runtime.Composable
import androidx.compose.ui.graphics.Color
import androidx.compose.ui.platform.LocalContext

/** Zenpi's own light palette (green primary, rust secondary). */
val ZenpiLight: ColorScheme = lightColorScheme(primary = Color(0xFF166A4C), secondary = Color(0xFF934A25),
    background = Color(0xFFF5F7F8), surface = Color.White)

/** The same hues at dark-theme tones on neutral green-grey surfaces. */
val ZenpiDark: ColorScheme = darkColorScheme(
    primary = Color(0xFF8BD5B0), onPrimary = Color(0xFF003825), primaryContainer = Color(0xFF005138), onPrimaryContainer = Color(0xFFA6F2CB),
    inversePrimary = Color(0xFF166A4C),
    secondary = Color(0xFFFFB693), onSecondary = Color(0xFF561F00), secondaryContainer = Color(0xFF763412), onSecondaryContainer = Color(0xFFFFDBCC),
    tertiary = Color(0xFFA5CCDF), onTertiary = Color(0xFF063546), tertiaryContainer = Color(0xFF244C5D), onTertiaryContainer = Color(0xFFC1E8FB),
    background = Color(0xFF0F1513), onBackground = Color(0xFFDEE4E0), surface = Color(0xFF0F1513), onSurface = Color(0xFFDEE4E0),
    surfaceVariant = Color(0xFF404944), onSurfaceVariant = Color(0xFFBFC9C2), surfaceTint = Color(0xFF8BD5B0),
    inverseSurface = Color(0xFFDEE4E0), inverseOnSurface = Color(0xFF2C3230), outline = Color(0xFF89938D), outlineVariant = Color(0xFF404944),
    surfaceBright = Color(0xFF353B38), surfaceDim = Color(0xFF0F1513), surfaceContainerLowest = Color(0xFF0A0F0D),
    surfaceContainerLow = Color(0xFF171D1B), surfaceContainer = Color(0xFF1B211F), surfaceContainerHigh = Color(0xFF252B29),
    surfaceContainerHighest = Color(0xFF303634))

/**
 * App theme. Follows the system dark setting; with [dynamic] (the default; minSdk 31 always supports it) the
 * colours come from the user's wallpaper, otherwise from Zenpi's palette (the goldens pin dynamic off).
 */
@Composable
fun ZenpiTheme(dark: Boolean = isSystemInDarkTheme(), dynamic: Boolean = true, content: @Composable () -> Unit) {
    val context = LocalContext.current
    val scheme = when {
        dynamic && dark -> dynamicDarkColorScheme(context)
        dynamic -> dynamicLightColorScheme(context)
        dark -> ZenpiDark
        else -> ZenpiLight
    }
    MaterialTheme(colorScheme = scheme, content = content)
}
