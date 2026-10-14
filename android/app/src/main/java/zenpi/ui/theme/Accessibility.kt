package zenpi.ui.theme

import androidx.compose.ui.Modifier
import androidx.compose.ui.semantics.clearAndSetSemantics
import androidx.compose.ui.semantics.contentDescription
import androidx.compose.ui.semantics.heading
import androidx.compose.ui.semantics.semantics

/** Marks a title so TalkBack announces it as a heading and can jump between headings. */
fun Modifier.asHeading(): Modifier = semantics { heading() }

/**
 * Shows an identifier (request, session or worker id, digest) without TalkBack reading it out:
 * the node is announced as [spoken] instead, or skipped when that is null.
 */
fun Modifier.spokenAs(spoken: String?): Modifier = clearAndSetSemantics { spoken?.let { contentDescription = it } }
