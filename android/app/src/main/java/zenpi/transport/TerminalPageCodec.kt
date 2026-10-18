package zenpi.transport

import com.google.gson.JsonObject
import com.google.gson.JsonNull
import java.math.BigInteger
import java.util.UUID

data class TerminalPage(val projectId: String, val sessionId: String, val terminalId: String, val subtabId: String,
    val subtabRevision: String, val digest: String, val offset: Int, val nextOffset: Int, val totalBytes: Int,
    val hasMore: Boolean, val text: String)

internal object TerminalPageCodec {
    private fun validPage(page: TerminalPage): Boolean {
        val bytes = page.text.toByteArray(Charsets.UTF_8).size
        return page.digest.matches(Regex("[0-9a-f]{64}")) && page.offset in 0..8 * 1024 * 1024
            && page.nextOffset in page.offset..8 * 1024 * 1024 && page.totalBytes in page.nextOffset..8 * 1024 * 1024
            && bytes <= 8192 && page.nextOffset - page.offset == bytes
            && page.hasMore == (page.nextOffset < page.totalBytes) && (!page.hasMore || bytes > 0)
            && page.text.none { Character.isISOControl(it) && it != '\n' && it != '\t' }
    }
    fun validChain(pages: List<TerminalPage>, current: TerminalPage): Boolean =
        pages.size <= 7 && validPage(current) && pages.all(::validPage)
            && pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            && pages.lastOrNull()?.let { consecutive(it, current) } != false
    fun terminal(projection: HostProjection?): RemoteTerminal? {
        if (!SessionCatalogCodec.readable(projection)) return null
        val project = projection?.projects?.singleOrNull { it.id == projection.selectedId } ?: return null
        val terminal = project.terminal?.takeIf(TerminalProjectionCodec::valid) ?: return null
        val tabs = SubtabProjectionCodec.current(projection) ?: return null
        return terminal.takeIf { it.terminalId != null && tabs.projectId == project.id && it.subtabId == tabs.activeId
            && it.subtabRevision == tabs.revision && it.projectId == project.id && it.sessionId == project.sessionId }
    }
    fun sameScope(previous: HostProjection, next: HostProjection?): Boolean {
        val first = terminal(previous) ?: return false
        val second = terminal(next) ?: return false
        return first.projectId == second.projectId && first.sessionId == second.sessionId && first.terminalId == second.terminalId
            && first.subtabId == second.subtabId && first.subtabRevision == second.subtabRevision
    }
    fun request(projection: HostProjection, previous: TerminalPage?): JsonObject {
        val terminal = requireNotNull(terminal(projection))
        if (previous != null) require(validPage(previous) && matches(previous, terminal) && previous.hasMore)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", terminal.projectId); addProperty("session_id", terminal.sessionId)
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "terminal_read")
                add("arguments", JsonObject().apply {
                    addProperty("terminal_id", terminal.terminalId); addProperty("subtab_id", terminal.subtabId)
                    addProperty("subtab_revision", BigInteger(terminal.subtabRevision)); addProperty("offset", previous?.nextOffset ?: 0)
                    add("digest", previous?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: JsonNull.INSTANCE)
                })
            })
        }
    }
    private fun matches(page: TerminalPage, terminal: RemoteTerminal) = page.projectId == terminal.projectId && page.sessionId == terminal.sessionId
        && page.terminalId == terminal.terminalId && page.subtabId == terminal.subtabId && page.subtabRevision == terminal.subtabRevision
    fun matchesSelection(page: TerminalPage, projection: HostProjection?): Boolean = validPage(page) && terminal(projection)?.let { matches(page, it) } == true
    fun decode(value: JsonObject, projection: HostProjection, previous: TerminalPage?): TerminalPage {
        request(projection, previous)
        val terminal = requireNotNull(terminal(projection))
        require(value.keySet() == setOf("project_id", "session_id", "terminal_id", "subtab_id", "subtab_revision", "digest",
            "offset", "next_offset", "total_bytes", "has_more", "text"))
        for ((field, expected) in listOf("project_id" to terminal.projectId, "session_id" to terminal.sessionId,
            "terminal_id" to terminal.terminalId, "subtab_id" to terminal.subtabId)) require(FrameCodec.string(value, field) == expected)
        val revision = value["subtab_revision"]
        require(revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString() == terminal.subtabRevision)
        fun number(field: String): Int {
            val n = value[field]
            require(n.isJsonPrimitive && n.asJsonPrimitive.isNumber && n.toString().matches(Regex("0|[1-9][0-9]*")))
            return requireNotNull(n.toString().toIntOrNull()?.takeIf { it in 0..8 * 1024 * 1024 })
        }
        val offset = number("offset"); val next = number("next_offset"); val total = number("total_bytes")
        val digest = FrameCodec.string(value, "digest"); val text = FrameCodec.string(value, "text")
        val flag = value["has_more"]
        require(flag.isJsonPrimitive && flag.asJsonPrimitive.isBoolean && digest.matches(Regex("[0-9a-f]{64}")))
        val bytes = text.toByteArray(Charsets.UTF_8).size
        require(bytes <= 8192 && text.none { Character.isISOControl(it) && it != '\n' && it != '\t' }
            && offset == (previous?.nextOffset ?: 0) && next - offset == bytes && next <= total
            && flag.asBoolean == (next < total) && (!flag.asBoolean || bytes > 0))
        if (previous != null) require(matches(previous, terminal) && previous.hasMore && previous.digest == digest && previous.totalBytes == total)
        return TerminalPage(terminal.projectId, terminal.sessionId, requireNotNull(terminal.terminalId), terminal.subtabId,
            terminal.subtabRevision, digest, offset, next, total, flag.asBoolean, text)
    }
    fun remember(pages: List<TerminalPage>, current: TerminalPage?, next: TerminalPage): List<TerminalPage> {
        if (current == null) { require(next.offset == 0 && validPage(next)); return emptyList() }
        require(validChain(pages, current) && validPage(next) && consecutive(current, next))
        return (pages + current).takeLast(7)
    }
    private fun consecutive(first: TerminalPage, second: TerminalPage): Boolean =
        first.copy(offset = second.offset, nextOffset = second.nextOffset, text = second.text, hasMore = second.hasMore) == second
            && first.hasMore && first.nextOffset == second.offset
    fun previous(pages: List<TerminalPage>, current: TerminalPage): TerminalPage? {
        val previous = pages.lastOrNull() ?: return null
        require(validChain(pages, current))
        return previous
    }
}
