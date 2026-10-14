package zenpi.ui.nav

import zenpi.ui.theme.asHeading
import androidx.compose.ui.Modifier
import androidx.compose.foundation.layout.PaddingValues
import androidx.compose.foundation.layout.RowScope
import androidx.compose.foundation.layout.WindowInsets
import androidx.compose.foundation.layout.WindowInsetsSides
import androidx.compose.foundation.layout.only
import androidx.compose.foundation.layout.safeDrawing
import androidx.compose.material.icons.Icons
import androidx.compose.material.icons.automirrored.outlined.ArrowBack
import androidx.compose.material3.ExperimentalMaterial3Api
import androidx.compose.material3.Icon
import androidx.compose.material3.IconButton
import androidx.compose.material3.Scaffold
import androidx.compose.material3.Text
import androidx.compose.material3.TopAppBar
import androidx.compose.foundation.layout.Column
import androidx.compose.runtime.Composable
import androidx.compose.runtime.staticCompositionLocalOf
import androidx.compose.ui.text.style.TextOverflow

/** Persistent content every page shows above its own bottom bar: the approval bar. */
val LocalPageBanner = staticCompositionLocalOf<@Composable () -> Unit> { {} }

/**
 * Frame of a full-screen secondary page: a top bar whose navigation icon goes back (described by
 * [back], e.g. "Close sessions"), optional [actions], the [LocalPageBanner] over an optional
 * [bottomBar], and the [content].
 */
@OptIn(ExperimentalMaterial3Api::class)
@Composable
fun SecondaryPage(title: String, back: String, onBack: () -> Unit, actions: @Composable RowScope.() -> Unit = {},
    bottomBar: @Composable () -> Unit = {}, content: @Composable (PaddingValues) -> Unit) {
    Scaffold(contentWindowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Horizontal),
        topBar = {
            TopAppBar(title = { Text(title, maxLines = 1, overflow = TextOverflow.Ellipsis, modifier = Modifier.asHeading()) },
                navigationIcon = { IconButton(onClick = onBack) { Icon(Icons.AutoMirrored.Outlined.ArrowBack, contentDescription = back) } },
                actions = actions, windowInsets = WindowInsets.safeDrawing.only(WindowInsetsSides.Top + WindowInsetsSides.Horizontal))
        }, bottomBar = { Column { LocalPageBanner.current(); bottomBar() } }, content = content)
}
