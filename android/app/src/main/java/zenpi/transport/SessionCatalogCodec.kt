package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

data class RemoteSession(val id: String, val createdAtMs: String, val turnCount: Int, val current: Boolean, val name: String? = null,
    val matches: Int = 0, val snippets: List<String> = emptyList())
data class SessionCatalog(val projectId: String, val sessionId: String, val snapshotId: String, val offset: Int,
    val nextOffset: Int?, val entries: List<RemoteSession>, val query: String? = null)

internal object SessionCatalogCodec {
    fun validIdentity(value: String): Boolean = value.isNotBlank()
        && value.toByteArray(Charsets.UTF_8).size <= 128 && value.none(Char::isISOControl)
    private fun validCatalog(catalog: SessionCatalog): Boolean {
        if (!catalog.snapshotId.matches(Regex("[0-9a-f]{64}")) || catalog.offset !in 0..496 || catalog.offset % 16 != 0
            || catalog.entries.size > 16 || catalog.offset > 0 && catalog.entries.isEmpty()
            || catalog.nextOffset?.let { catalog.entries.size != 16 || it != catalog.offset + 16 || it >= 512 } == true
            || catalog.entries.map { it.id }.distinct().size != catalog.entries.size
            || catalog.query?.let { runCatching { validateQuery(it) }.isFailure } == true) return false
        return catalog.entries.all { entry ->
            validIdentity(entry.id) && entry.current == (entry.id == catalog.sessionId)
                && entry.createdAtMs.matches(Regex("0|[1-9][0-9]*"))
                && entry.createdAtMs.toBigInteger() <= BigInteger("18446744073709551615")
                && entry.turnCount >= 0 && entry.matches in 0..entry.turnCount
                && entry.snippets.size == minOf(entry.matches, 4)
                && (catalog.query != null || entry.matches == 0 && entry.snippets.isEmpty())
                && entry.snippets.all { text -> text.codePointCount(0, text.length) <= 240
                    && text.toByteArray(Charsets.UTF_8).size <= 960 && text.none { it.isISOControl() && it != '\n' && it != '\t' } }
                && (entry.name == null || runCatching { SessionCommandCodec.validateName(entry.name) }.isSuccess)
        }
    }
    fun validChain(pages: List<SessionCatalog>, current: SessionCatalog): Boolean {
        if (pages.size > 7 || !validCatalog(current) || !pages.all(::validCatalog)
            || !pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val ids = (pages + current).flatMap { it.entries }.map { it.id }
        return ids.distinct().size == ids.size
    }
    fun readable(projection: HostProjection?): Boolean = projection != null && projection.selectedId != null && projection.sessionId != null
        && projection.projects.singleOrNull { it.id == projection.selectedId }?.sessionId == projection.sessionId
    fun sameScope(catalog: SessionCatalog, projection: HostProjection?) = validCatalog(catalog) && projection != null && readable(projection)
        && catalog.projectId == projection.selectedId && catalog.sessionId == projection.sessionId
        && projection.projects.singleOrNull { it.id == catalog.projectId && it.sessionId == catalog.sessionId } != null
    fun resumable(catalog: SessionCatalog, projection: HostProjection?, target: String): Boolean =
        sameScope(catalog, projection) && target != catalog.sessionId
            && catalog.entries.singleOrNull { it.id == target }?.current == false
    private fun consecutive(first: SessionCatalog, second: SessionCatalog): Boolean =
        first.projectId == second.projectId && first.sessionId == second.sessionId
            && first.snapshotId == second.snapshotId && first.query == second.query && first.nextOffset == second.offset

    fun validateQuery(query: String): String {
        require(query == query.trim() && query.toByteArray(Charsets.UTF_8).size <= 128 && query.none(Char::isISOControl))
        return query
    }

    fun remember(pages: List<SessionCatalog>, current: SessionCatalog?, next: SessionCatalog): List<SessionCatalog> {
        if (current == null) { require(validCatalog(next) && next.offset == 0); return emptyList() }
        require(validChain(pages, current) && validCatalog(next))
        if (next.offset < current.offset) {
            require(pages.lastOrNull() == next)
            return pages.dropLast(1)
        }
        require(consecutive(current, next))
        val retained = pages + current
        require(retained.flatMap { it.entries }.none { old -> next.entries.any { it.id == old.id } })
        return retained.takeLast(7)
    }

    private fun requireOffset(projection: HostProjection, previous: SessionCatalog?, offset: Int) {
        require(readable(projection) && offset in 0..496 && offset % 16 == 0)
        require(previous == null && offset == 0 || previous != null && sameScope(previous, projection)
            && (offset == previous.nextOffset || previous.offset > 0 && offset == previous.offset - 16))
    }
    fun request(projection: HostProjection, previous: SessionCatalog?, offset: Int, query: String? = previous?.query): JsonObject {
        requireOffset(projection, previous, offset)
        query?.let(::validateQuery)
        require(previous == null || previous.query == query)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "session_list"); add("arguments", JsonObject().apply {
                if (previous != null) { addProperty("offset", offset); addProperty("snapshot_id", previous.snapshotId) }
                query?.let { addProperty("query", it) }
            }) })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, previous: SessionCatalog?, offset: Int, query: String? = previous?.query): SessionCatalog {
        request(projection, previous, offset, query)
        require(value.keySet() == setOf("project_id", "session_id", "snapshot_id", "offset", "next_offset", "entries") + if (query != null) setOf("query") else emptySet())
        if (query != null) require(FrameCodec.string(value, "query") == query)
        require(FrameCodec.string(value, "project_id") == projection.selectedId && FrameCodec.string(value, "session_id") == projection.sessionId)
        val digest = FrameCodec.string(value, "snapshot_id"); require(digest.matches(Regex("[0-9a-f]{64}")))
        fun number(obj: JsonObject, name: String): BigInteger {
            val n = obj[name]; require(n.isJsonPrimitive && n.asJsonPrimitive.isNumber && n.toString().matches(Regex("0|[1-9][0-9]*"))
                && n.asBigInteger <= BigInteger("18446744073709551615")); return n.asBigInteger
        }
        require(offset in 0..512 && offset % 16 == 0 && number(value, "offset") == BigInteger.valueOf(offset.toLong()))
        if (previous != null) require(sameScope(previous, projection) && previous.snapshotId == digest)
        val items = value["entries"]; require(items.isJsonArray && items.asJsonArray.size() <= 16)
        val entries = items.asJsonArray.map { item ->
            require(item.isJsonObject); val entry = item.asJsonObject
            val fields = setOf("session_id", "created_at_ms", "turn_count", "current")
            val searchFields = if (entry.has("matches") || entry.has("snippets")) setOf("matches", "snippets") else emptySet()
            require(searchFields.isEmpty() || query != null)
            require(entry.keySet() == fields + searchFields || entry.keySet() == fields + searchFields + "name")
            val id = FrameCodec.string(entry, "session_id"); require(validIdentity(id))
            val flag = entry["current"]; require(flag.isJsonPrimitive && flag.asJsonPrimitive.isBoolean && flag.asBoolean == (id == projection.sessionId))
            val count = number(entry, "turn_count"); require(count <= BigInteger.valueOf(Int.MAX_VALUE.toLong()))
            val matches = if (searchFields.isEmpty()) 0 else number(entry, "matches").intValueExact().also { require(it >= 0 && BigInteger.valueOf(it.toLong()) <= count) }
            val snippets = if (searchFields.isEmpty()) emptyList() else {
                val values = entry["snippets"]; require(values.isJsonArray && values.asJsonArray.size() == minOf(matches, 4))
                values.asJsonArray.map { value ->
                    require(value.isJsonPrimitive && value.asJsonPrimitive.isString)
                    value.asString.also { text -> require(text.codePointCount(0, text.length) <= 240
                        && text.toByteArray(Charsets.UTF_8).size <= 960 && text.none { it.isISOControl() && it != '\n' && it != '\t' }) }
                }
            }
            RemoteSession(id, number(entry, "created_at_ms").toString(), count.toInt(), flag.asBoolean,
                entry["name"]?.takeUnless { it.isJsonNull }?.let { SessionCommandCodec.validateName(FrameCodec.string(entry, "name")) }, matches, snippets)
        }
        require(entries.map { it.id }.distinct().size == entries.size)
        require(offset == 0 || entries.isNotEmpty())
        require(previous == null || previous.entries.none { old -> entries.any { it.id == old.id } })
        val next = if (value["next_offset"].isJsonNull) null else number(value, "next_offset").intValueExact()
        require(next == null || entries.size == 16 && next == offset + 16 && next < 512)
        return SessionCatalog(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), digest, offset, next, entries, query)
            .also { require(validCatalog(it)) }
    }
}
