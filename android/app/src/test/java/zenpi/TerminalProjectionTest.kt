package zenpi

import com.google.gson.*
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.TerminalProjectionCodec

class TerminalProjectionTest {
    private fun value() = JsonParser.parseString("""{"project_id":"project","session_id":"session","subtab_id":"${"a".repeat(64)}","subtab_revision":7,"text":"visible\n","truncated":false,"read_only":true}""").asJsonObject
    @Test fun exactReadOnlyScopeAndAbsentStateAreAccepted() {
        assertNull(TerminalProjectionCodec.decode(null, "project", "session"))
        assertNull(TerminalProjectionCodec.decode(JsonNull.INSTANCE, "project", "session"))
        assertEquals("visible\n", TerminalProjectionCodec.decode(value(), "project", "session")?.text)
        assertEquals("b".repeat(64), TerminalProjectionCodec.decode(value().apply { addProperty("terminal_id", "b".repeat(64)) }, "project", "session")?.terminalId)
    }
    @Test fun wrongScopeControlTypesTextAndOverflowAreRefused() {
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("project_id", "foreign") }, { it.addProperty("session_id", "foreign") },
            { it.addProperty("subtab_id", "invalid") }, { it.addProperty("subtab_revision", "7") },
            { it.addProperty("subtab_revision", -1) }, { it.addProperty("subtab_revision", java.math.BigInteger("18446744073709551616")) },
            { it.addProperty("read_only", false) }, { it.addProperty("truncated", "false") },
            { it.addProperty("text", "x".repeat(16385)) }, { it.addProperty("text", "bad\u001b") },
            { it.addProperty("root", "/private") })) {
            val bad = value(); change(bad)
            assertTrue(runCatching { TerminalProjectionCodec.decode(bad, "project", "session") }.isFailure)
        }
        for (id in listOf("", "b".repeat(63), "B".repeat(64), "foreign")) {
            assertTrue(runCatching { TerminalProjectionCodec.decode(value().apply { addProperty("terminal_id", id) }, "project", "session") }.isFailure)
        }
    }
    @Test fun actualDimensionsAreOptionalButExactBoundedAndInstanceOwned() {
        val item = value().apply { addProperty("terminal_id", "b".repeat(64)) }
        item.add("size", JsonParser.parseString("""{"rows":33,"cols":91}"""))
        val size = requireNotNull(TerminalProjectionCodec.decode(item, "project", "session")?.size)
        assertEquals(33, size.rows); assertEquals(91, size.cols)
        for (raw in listOf("null", "[]", "{}", "{\"rows\":33}", "{\"rows\":33,\"cols\":91,\"extra\":0}",
            "{\"rows\":0,\"cols\":91}", "{\"rows\":33,\"cols\":513}", "{\"rows\":\"33\",\"cols\":91}",
            "{\"rows\":33.0,\"cols\":91}", "{\"rows\":true,\"cols\":91}", "{\"rows\":18446744073709551616,\"cols\":91}")) {
            val invalid = item.deepCopy().apply { add("size", JsonParser.parseString(raw)) }
            assertTrue(raw, runCatching { TerminalProjectionCodec.decode(invalid, "project", "session") }.isFailure)
        }
        assertTrue(runCatching { TerminalProjectionCodec.decode(item.deepCopy().apply { remove("terminal_id") }, "project", "session") }.isFailure)
        val bounds = item.deepCopy().apply { add("size", JsonParser.parseString("""{"rows":1,"cols":512}""")) }
        assertEquals(512, TerminalProjectionCodec.decode(bounds, "project", "session")?.size?.cols)
    }
    @Test fun exitedLifecycleIsBooleanAndRequiresAnInstance() {
        val item = value().apply { addProperty("terminal_id", "b".repeat(64)); addProperty("running", false) }
        val terminal = requireNotNull(TerminalProjectionCodec.decode(item, "project", "session"))
        assertFalse(terminal.running); assertEquals("visible\n", terminal.text)
        assertTrue(TerminalProjectionCodec.decode(value(), "project", "session")!!.running)
        for (raw in listOf("null", "\"false\"", "0", "{}", "[]")) {
            assertTrue(runCatching { TerminalProjectionCodec.decode(item.deepCopy().apply { add("running", JsonParser.parseString(raw)) }, "project", "session") }.isFailure)
        }
        assertTrue(runCatching { TerminalProjectionCodec.decode(item.deepCopy().apply { remove("terminal_id") }, "project", "session") }.isFailure)
    }
}
