package zenpi

import androidx.compose.ui.text.SpanStyle
import androidx.compose.ui.text.font.FontFamily
import org.junit.Assert.assertEquals
import org.junit.Test
import zenpi.transport.RemotePromptStatus
import zenpi.ui.ComposerMode
import zenpi.ui.Markdown
import zenpi.ui.MarkdownBlock.Code
import zenpi.ui.MarkdownBlock.Item
import zenpi.ui.MarkdownBlock.Paragraph

class MarkdownTest {
    @Test fun blocksCoverParagraphsFencedCodeAndNestedLists() {
        val text = "Plan:\nfirst line kept\n\n- read `a.rs`\n  * nested\n2) second\n10. tenth\n" +
            "```rust\nfn main() {}\n  indented\n```\nafter"
        assertEquals(listOf(Paragraph("Plan:\nfirst line kept"), Item("•", "read `a.rs`", 0), Item("•", "nested", 1),
            Item("2)", "second", 0), Item("10.", "tenth", 0), Code("rust", "fn main() {}\n  indented", true), Paragraph("after")),
            Markdown.parse(text))
    }

    @Test fun streamedOpenFenceAndUnsupportedSyntaxNeverHideText() {
        assertEquals(listOf(Paragraph("x"), Code("", "partial\n", false)), Markdown.parse("x\n```\npartial\n"))
        assertEquals(listOf(Paragraph("# Title\n**bold** [link](u)\n-not a list\n1.not either")),
            Markdown.parse("# Title\n**bold** [link](u)\n-not a list\n1.not either"))
        assertEquals(listOf(Paragraph("crlf")), Markdown.parse("crlf\r\n"))
        assertEquals(emptyList<Any>(), Markdown.parse(""))
    }

    @Test fun inlineCodeIsStyledAndUnmatchedBackticksStayLiteral() {
        val code = SpanStyle(fontFamily = FontFamily.Monospace)
        val styled = Markdown.inline("call `f()` then `g`", code)
        assertEquals("call f() then g", styled.text)
        assertEquals(listOf(5 to 8, 14 to 15), styled.spanStyles.map { it.start to it.end })
        assertEquals("a `b", Markdown.inline("a `b", code).text)
        assertEquals("empty `` pair", Markdown.inline("empty `` pair", code).text)
        assertEquals(emptyList<Any>(), Markdown.inline("empty `` pair", code).spanStyles)
    }

    @Test fun composerSendsWhenIdleStopsWhenEmptyAndSteersWithText() {
        val running = RemotePromptStatus("r", "p", "s", "running")
        assertEquals(ComposerMode.SEND, ComposerMode.of(null, ""))
        assertEquals(ComposerMode.SEND, ComposerMode.of(null, "hi"))
        assertEquals(ComposerMode.STOP, ComposerMode.of(running, " \n"))
        assertEquals(ComposerMode.STEER, ComposerMode.of(running, "go left"))
    }
}
