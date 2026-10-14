package zenpi

import com.google.gson.JsonNull
import com.google.gson.JsonPrimitive
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.DesktopStreamCodec

class DesktopStreamTest {
    @Test fun absentPreviewIsCompatibleAndTextIsByteBounded() {
        assertNull(DesktopStreamCodec.decode(null)); assertNull(DesktopStreamCodec.decode(JsonNull.INSTANCE))
        assertEquals("partial", DesktopStreamCodec.decode(JsonPrimitive("partial")))
        assertEquals(4096, requireNotNull(DesktopStreamCodec.decode(JsonPrimitive("x".repeat(4096)))).length)
        for (bad in listOf(JsonPrimitive(true), JsonPrimitive(1), JsonPrimitive("x".repeat(4097)), JsonPrimitive("bad\u0000"), JsonPrimitive("界".repeat(1366)))) {
            assertTrue(runCatching { DesktopStreamCodec.decode(bad) }.isFailure)
        }
    }
}
