package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.SteeringInput

class SteeringInputTest {
    @Test fun queueTextRejectsCommandsAndAttachmentTokensButNotLiteralInlineText() {
        for (text in listOf("", "  ", "\u0085\u00a0", "/help", "\u0085/skill:test", "\u00a0!command", "@", "@\"unfinished",
            "use @file.txt", "use\n@file", "use\u0085@file", "use\u00a0@file", "\u4e2d @file", "x\u0000y", "x".repeat(65537)))
            assertFalse(text, SteeringInput.valid(text))
        for (text in listOf("plain text", "\u4e2d\u6587", "a@example.invalid", "(@file)", "inline /skill:test", "inline !command",
            "x\u001c@literal", "\ud83d\ude00@literal", "x".repeat(65536), "\u4e2d".repeat(21845)))
            assertTrue(text, SteeringInput.valid(text))
        assertFalse(SteeringInput.valid("\u4e2d".repeat(21846)))
    }
}
