package zenpi

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.PromptStatusCodec

class PromptStatusTest {
    private fun status() = JsonObject().apply {
        addProperty("request_id", "request"); addProperty("project_id", "project")
        addProperty("session_id", "session"); addProperty("state", "succeeded")
    }
    @Test fun typedFailureIsOnlyOnFailedStatusAndNeverAcceptsRawDiagnostics() {
        val failed=status().apply{addProperty("state","failed");addProperty("failure_code","invalid_request")}
        assertEquals("invalid_request",PromptStatusCodec.decode(JsonArray().apply{add(failed)},"project","session").single().failureCode)
        for(change in listOf<(JsonObject)->Unit>({it.addProperty("failure_code","/private/host/error")},
            {it.addProperty("failure_code",true)}, {it.add("failure_code",com.google.gson.JsonNull.INSTANCE)},
            {it.addProperty("state","running")},{it.addProperty("state","succeeded")},
            {it.addProperty("message","Bearer PRIVATE_ERROR")})) {
            assertTrue(runCatching{PromptStatusCodec.decode(JsonArray().apply{add(failed.deepCopy().apply(change))},"project","session")}.isFailure)
        }
    }
    @Test fun boundedStatusRequiresExactOwnerStateAndIdentity() {
        val values = JsonArray().apply { add(status()) }
        assertEquals("succeeded", PromptStatusCodec.decode(values, "project", "session").single().state)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("state", "complete") }, { it.addProperty("project_id", "foreign") },
            { it.addProperty("session_id", "foreign") }, { it.addProperty("request_id", "bad\n") },
            { it.addProperty("private_path", "/private") })) {
            val item = status(); change(item)
            assertTrue(runCatching { PromptStatusCodec.decode(JsonArray().apply { add(item) }, "project", "session") }.isFailure)
        }
        assertTrue(runCatching { PromptStatusCodec.decode(JsonArray().apply { add(status()); add(status()) }, "project", "session") }.isFailure)
        assertTrue(runCatching { PromptStatusCodec.decode(JsonArray().apply { repeat(17) { add(status().apply { addProperty("request_id", "r-$it") }) } }, "project", "session") }.isFailure)
    }
    @Test fun liveTextIsBoundedTransientAndOnlyOnActiveStatus() {
        val value = status().apply { addProperty("state", "running"); addProperty("stream_text", "partial\n") }
        assertEquals("partial\n", PromptStatusCodec.decode(JsonArray().apply { add(value) }, "project", "session").single().streamText)
        for (change in listOf<(JsonObject) -> Unit>({ it.addProperty("state", "succeeded") },
            { it.addProperty("stream_text", "x".repeat(4097)) }, { it.addProperty("stream_text", "bad\u0000") },
            { it.addProperty("stream_text", true) })) {
            val bad = value.deepCopy(); change(bad)
            assertTrue(runCatching { PromptStatusCodec.decode(JsonArray().apply { add(bad) }, "project", "session") }.isFailure)
        }
    }
    @Test fun truncationIsExplicitBooleanAndCannotAppearWithoutActivePreview() {
        val value = status().apply {
            addProperty("state", "running"); addProperty("stream_text", "safe prefix\n"); addProperty("stream_truncated", true)
        }
        val decoded = PromptStatusCodec.decode(JsonArray().apply { add(value) }, "project", "session").single()
        assertEquals("safe prefix\n", decoded.streamText); assertTrue(decoded.streamTruncated)
        for (change in listOf<(JsonObject) -> Unit>(
            { it.addProperty("stream_truncated", "true") }, { it.addProperty("stream_truncated", 1) },
            { it.addProperty("stream_truncated", false) }, { it.remove("stream_text") },
            { it.addProperty("state", "succeeded") })) {
            val bad = value.deepCopy(); change(bad)
            assertTrue(runCatching { PromptStatusCodec.decode(JsonArray().apply { add(bad) }, "project", "session") }.isFailure)
        }
        value.addProperty("stream_text", "")
        assertTrue(PromptStatusCodec.decode(JsonArray().apply { add(value) }, "project", "session").single().streamTruncated)
    }
}
