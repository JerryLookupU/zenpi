package zenpi.ui

import androidx.compose.foundation.horizontalScroll
import androidx.compose.foundation.layout.Arrangement
import androidx.compose.foundation.layout.Column
import androidx.compose.foundation.layout.Row
import androidx.compose.foundation.layout.fillMaxWidth
import androidx.compose.foundation.layout.padding
import androidx.compose.foundation.rememberScrollState
import androidx.compose.material3.MaterialTheme
import androidx.compose.material3.Surface
import androidx.compose.material3.Text
import androidx.compose.runtime.Composable
import androidx.compose.runtime.Immutable
import androidx.compose.runtime.remember
import androidx.compose.ui.Modifier
import androidx.compose.ui.text.AnnotatedString
import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.buildAnnotatedString
import androidx.compose.ui.text.font.FontFamily
import androidx.compose.ui.text.withStyle
import androidx.compose.ui.unit.dp

/** One block of the light markdown subset the conversation renders. */
@Immutable
sealed interface MarkdownBlock {
    /** Consecutive non-blank lines; line breaks inside are kept, as chat output relies on them. */
    data class Paragraph(val text: String) : MarkdownBlock
    /** A fenced block; [closed] is false while a streamed fence is still open. */
    data class Code(val language: String, val text: String, val closed: Boolean) : MarkdownBlock
    /** A `-`, `*`, `+` or `1.` / `1)` item; [marker] is "•" or the number with its delimiter. */
    data class Item(val marker: String, val text: String, val depth: Int) : MarkdownBlock
}

/**
 * Light markdown with no third-party parser: fenced code blocks, inline code and lists. Anything
 * else (headings, emphasis, links, tables) is shown verbatim, so no text is ever hidden.
 */
object Markdown {
    private val item = Regex("""^( *)([-*+]|\d{1,9}[.)])[ \t]+(.*)$""")

    fun parse(text: String): List<MarkdownBlock> {
        val blocks = mutableListOf<MarkdownBlock>()
        val paragraph = mutableListOf<String>()
        fun flush() { if (paragraph.isNotEmpty()) { blocks += MarkdownBlock.Paragraph(paragraph.joinToString("\n")); paragraph.clear() } }
        val lines = text.split('\n')
        var index = 0
        while (index < lines.size) {
            val line = lines[index].removeSuffix("\r")
            val fence = line.trimStart()
            if (fence.startsWith("```")) {
                flush()
                val body = mutableListOf<String>()
                index++
                while (index < lines.size && !lines[index].trimStart().startsWith("```")) body += lines[index++].removeSuffix("\r")
                blocks += MarkdownBlock.Code(fence.removePrefix("```").trim(), body.joinToString("\n"), index < lines.size)
                index++
                continue
            }
            val match = item.matchEntire(line)
            when {
                line.isBlank() -> flush()
                match != null -> {
                    flush()
                    val (indent, marker, body) = match.destructured
                    blocks += MarkdownBlock.Item(if (marker[0].isDigit()) marker else "•", body, indent.length / 2)
                }
                else -> paragraph += line
            }
            index++
        }
        flush()
        return blocks
    }

    /** Backtick spans become [code]; an unmatched backtick stays literal. */
    fun inline(text: String, code: SpanStyle): AnnotatedString = buildAnnotatedString {
        var at = 0
        while (at < text.length) {
            val open = text.indexOf('`', at)
            val close = if (open < 0) -1 else text.indexOf('`', open + 1)
            if (open < 0 || close < 0) { append(text.substring(at)); break }
            append(text.substring(at, open))
            if (close == open + 1) append("``") else withStyle(code) { append(text.substring(open + 1, close)) }
            at = close + 1
        }
    }
}

/** Renders [text] as [Markdown]; code blocks scroll horizontally instead of wrapping. */
@Composable
fun MarkdownText(text: String, modifier: Modifier = Modifier) {
    val blocks = remember(text) { Markdown.parse(text) }
    val codeColor = MaterialTheme.colorScheme.surfaceVariant
    val code = remember(codeColor) { SpanStyle(fontFamily = FontFamily.Monospace, background = codeColor) }
    val body = MaterialTheme.typography.bodyMedium
    Column(modifier, verticalArrangement = Arrangement.spacedBy(6.dp)) {
        for (block in blocks) when (block) {
            is MarkdownBlock.Paragraph -> Text(Markdown.inline(block.text, code), style = body)
            is MarkdownBlock.Item -> Row(Modifier.fillMaxWidth().padding(start = (12 * block.depth).dp)) {
                Text(block.marker, style = body, modifier = Modifier.padding(end = 8.dp))
                Text(Markdown.inline(block.text, code), style = body)
            }
            is MarkdownBlock.Code -> Surface(color = codeColor, shape = MaterialTheme.shapes.small, modifier = Modifier.fillMaxWidth()) {
                Text(block.text, style = MaterialTheme.typography.bodySmall, fontFamily = FontFamily.Monospace, softWrap = false,
                    color = MaterialTheme.colorScheme.onSurfaceVariant,
                    modifier = Modifier.horizontalScroll(rememberScrollState()).padding(8.dp))
            }
        }
    }
}
