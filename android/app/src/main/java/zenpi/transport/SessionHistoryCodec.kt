package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

data class SessionHistoryPage(val projectId: String, val sessionId: String, val journalRevision: String,
    val digest: String, val offset: Int, val nextOffset: Int, val totalTurns: Int, val hasMore: Boolean, val turns: List<RemoteTurn>)

internal object SessionHistoryCodec {
    private fun identifier(value: String): Boolean = value.isNotBlank()
        && value.toByteArray(Charsets.UTF_8).size <= 128 && value.none(Char::isISOControl)
    fun readable(projection: HostProjection?): Boolean = projection != null
        && SessionCatalogCodec.readable(projection)
        && projection.journalSequence?.let { revision ->
            revision.matches(Regex("0|[1-9][0-9]*"))
                && runCatching { BigInteger(revision) <= BigInteger("18446744073709551615") }.getOrDefault(false)
        } == true
    private fun validPage(page: SessionHistoryPage): Boolean = page.digest.matches(Regex("[0-9a-f]{64}"))
        && page.journalRevision.matches(Regex("0|[1-9][0-9]*"))
        && runCatching { BigInteger(page.journalRevision) <= BigInteger("18446744073709551615") }.getOrDefault(false)
        && page.totalTurns in 0..16384 && page.offset in 0..page.totalTurns && page.nextOffset in page.offset..page.totalTurns
        && page.turns.size <= 16 && page.nextOffset - page.offset == page.turns.size
        && page.hasMore == (page.nextOffset < page.totalTurns) && (!page.hasMore || page.turns.isNotEmpty())
        && (page.totalTurns == 0 || page.turns.isNotEmpty())
        && page.turns.all { identifier(it.id) && it.role in setOf("system", "user", "assistant", "tool")
            && it.content.toByteArray(Charsets.UTF_8).size <= 2048 && !it.content.contains('\u0000')
            && ToolCallProjectionCodec.valid(it) }

    private fun consecutive(a: SessionHistoryPage, b: SessionHistoryPage) = a.projectId == b.projectId && a.sessionId == b.sessionId
        && a.journalRevision == b.journalRevision && a.digest == b.digest && a.totalTurns == b.totalTurns && a.hasMore && a.nextOffset == b.offset

    fun validChain(pages: List<SessionHistoryPage>, current: SessionHistoryPage): Boolean {
        if (pages.size > 7 || !validPage(current) || !pages.all(::validPage)
            || !pages.zipWithNext().all { (a, b) -> consecutive(a, b) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val ids = (pages + current).flatMap { it.turns }.map { it.id }
        return ids.distinct().size == ids.size
    }
    fun sameScope(page: SessionHistoryPage, projection: HostProjection?): Boolean = projection != null && readable(projection)
        && page.projectId == projection.selectedId && page.sessionId == projection.sessionId
        && page.journalRevision == projection.journalSequence
        && projection.projects.singleOrNull { it.id == page.projectId && it.sessionId == page.sessionId } != null

    fun request(projection: HostProjection, previous: SessionHistoryPage?): JsonObject {
        require(readable(projection))
        require(previous == null || sameScope(previous, projection) && previous.hasMore && validChain(emptyList(), previous))
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "transcript_read")
                add("arguments", JsonObject().apply {
                    addProperty("view", "session_history"); addProperty("offset", previous?.nextOffset ?: 0)
                    addProperty("journal_revision", BigInteger(requireNotNull(projection.journalSequence)))
                    add("digest", previous?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE)
                })
            })
        }
    }

    fun decode(value: JsonObject, projection: HostProjection, previous: SessionHistoryPage?): SessionHistoryPage {
        request(projection, previous)
        require(value.keySet() == setOf("project_id", "session_id", "journal_revision", "digest", "offset", "next_offset", "total_turns", "has_more", "turns"))
        require(FrameCodec.string(value, "project_id") == projection.selectedId && FrameCodec.string(value, "session_id") == projection.sessionId)
        val revision = value["journal_revision"]
        require(revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString() == projection.journalSequence)
        fun number(name: String): Int = value[name].let {
            require(it.isJsonPrimitive && it.asJsonPrimitive.isNumber && it.toString().matches(Regex("0|[1-9][0-9]*")))
            it.toString().toInt().also { n -> require(n in 0..16384) }
        }
        val offset = number("offset"); val next = number("next_offset"); val total = number("total_turns")
        val digest = FrameCodec.string(value, "digest"); require(digest.matches(Regex("[0-9a-f]{64}")))
        val flag = value["has_more"]; require(flag.isJsonPrimitive && flag.asJsonPrimitive.isBoolean)
        val items = value["turns"]; require(items.isJsonArray && items.asJsonArray.size() <= 16)
        val turns = items.asJsonArray.map { raw ->
            require(raw.isJsonObject); val item = raw.asJsonObject
            val fields = setOf("id", "role", "content", "content_truncated")
            require(item.keySet() == fields || item.keySet() == fields + setOf("tool_calls", "tool_calls_truncated"))
            val id = FrameCodec.string(item, "id"); val role = FrameCodec.string(item, "role"); val text = FrameCodec.string(item, "content")
            require(identifier(id) && role in setOf("system", "user", "assistant", "tool")
                && text.toByteArray(Charsets.UTF_8).size <= 2048 && !text.contains('\u0000'))
            val truncated = item["content_truncated"]; require(truncated.isJsonPrimitive && truncated.asJsonPrimitive.isBoolean)
            RemoteTurn(id, role, text, truncated.asBoolean, ToolCallProjectionCodec.decode(item, role), item.get("tool_calls_truncated")?.asBoolean ?: false)
        }
        require(offset == (previous?.nextOffset ?: 0) && next - offset == turns.size && next <= total
            && flag.asBoolean == (next < total) && (!flag.asBoolean || turns.isNotEmpty())
            && (total == 0 || turns.isNotEmpty())
            && turns.map { it.id }.distinct().size == turns.size
            && (previous == null || previous.digest == digest && previous.totalTurns == total
                && previous.turns.none { old -> turns.any { it.id == old.id } }))
        return SessionHistoryPage(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), requireNotNull(projection.journalSequence),
            digest, offset, next, total, flag.asBoolean, turns).also { require(validPage(it)) }
    }

    fun remember(pages: List<SessionHistoryPage>, current: SessionHistoryPage?, next: SessionHistoryPage): List<SessionHistoryPage> {
        if (current == null) { require(next.offset == 0 && validChain(emptyList(), next)); return emptyList() }
        require(validChain(pages, current) && validPage(next) && consecutive(current, next))
        require((pages + current).flatMap { it.turns }.none { old -> next.turns.any { it.id == old.id } })
        return (pages + current).takeLast(7)
    }
}
