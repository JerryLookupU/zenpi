package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

data class RemoteDiff(val projectId: String, val sessionId: String, val subtabId: String,
    val subtabRevision: String, val path: String, val changed: Boolean, val binary: Boolean,
    val truncated: Boolean, val diff: String, val status: String, val page: DiffPage? = null)
data class DiffPage(val digest: String, val offset: Int, val nextOffset: Int, val totalBytes: Int, val hasMore: Boolean)

internal object DiffProjectionCodec {
    private fun validatePath(path: String) {
        if (path.isBlank() || path.toByteArray().size > 4096 || path.any { Character.isISOControl(it) }
            || path.startsWith("/") || path.split('/').any { it == ".." } || WorkspaceFilesCodec.privatePath(path))
            throw WireException("Invalid diff path")
    }
    fun readable(projection: HostProjection?): Boolean = projection != null && SessionCatalogCodec.readable(projection)
        && projection.projects.singleOrNull { it.id == projection.selectedId }?.diffAvailable == true
        && SubtabProjectionCodec.current(projection) != null
    private fun validDocument(diff: RemoteDiff): Boolean {
        if (runCatching { validatePath(diff.path); require(diff.path == normalized(diff.path)) }.isFailure
            || diff.status.toByteArray(Charsets.UTF_8).size > 32768) return false
        val bytes = diff.diff.toByteArray(Charsets.UTF_8).size
        val page = diff.page ?: return bytes <= 65536
        return page.digest.matches(Regex("[0-9a-f]{64}")) && page.offset in 0..1048576
            && page.nextOffset in page.offset..1048576 && page.totalBytes in page.nextOffset..1048576
            && bytes <= 8192 && page.nextOffset - page.offset == bytes
            && page.hasMore == (page.nextOffset < page.totalBytes) && (!page.hasMore || bytes > 0)
    }
    fun validChain(pages: List<RemoteDiff>, current: RemoteDiff): Boolean = pages.size <= 7 && validDocument(current)
        && pages.all(::validDocument) && pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
        && pages.lastOrNull()?.let { consecutive(it, current) } != false
    fun sameSelection(diff: RemoteDiff, projection: HostProjection?): Boolean = projection != null && readable(projection) && validDocument(diff)
        && diff.projectId == projection.selectedId && diff.sessionId == projection.sessionId
        && projection.projects.singleOrNull { it.id == diff.projectId && it.sessionId == diff.sessionId }?.diffAvailable == true
        && projection.subtabs?.let { it.projectId == diff.projectId && it.activeId == diff.subtabId && it.revision == diff.subtabRevision
            && it.tabs.singleOrNull { tab -> tab.id == it.activeId } != null } == true
    private fun normalized(path: String) = path.split('/').filter { it.isNotEmpty() && it != "." }.joinToString("/").ifEmpty { "." }
    fun request(projection: HostProjection, path: String, previous: RemoteDiff? = null, paged: Boolean = false): JsonObject {
        require(readable(projection))
        val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId }?.takeIf { it.diffAvailable }
            ?: throw WireException("Diff unavailable")
        val subtabs = SubtabProjectionCodec.current(projection) ?: throw WireException("Missing diff selection")
        validatePath(path)
        if (previous != null) require(paged && sameSelection(previous, projection) && previous.path == normalized(path) && previous.page?.hasMore == true)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "diff")
                add("arguments", JsonObject().apply { addProperty("path", path); addProperty("subtab_id", subtabs.activeId)
                    addProperty("subtab_revision", BigInteger(subtabs.revision))
                    if (paged) { addProperty("offset", previous?.page?.nextOffset ?: 0)
                        add("digest", previous?.page?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE) }
                })
            })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, requestedPath: String, previous: RemoteDiff? = null, paged: Boolean = false): RemoteDiff {
        request(projection, requestedPath, previous, paged)
        validatePath(requestedPath)
        val subtabs = projection.subtabs ?: throw WireException("Missing diff selection")
        if (value.keySet() != setOf("project_id", "session_id", "subtab_id", "subtab_revision", "path", "changed", "binary", "truncated", "diff", "status") + (if (paged) setOf("page") else emptySet())
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId
            || FrameCodec.string(value, "subtab_id") != subtabs.activeId) throw WireException("Diff owner mismatch")
        val revision = value["subtab_revision"]
        if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || revision.toString() != subtabs.revision)
            throw WireException("Diff revision mismatch")
        val path = FrameCodec.string(value, "path")
        validatePath(path)
        val expected = normalized(requestedPath)
        if (path != expected) throw WireException("Diff path mismatch")
        fun flag(field: String): Boolean {
            val flag = value[field]
            if (!flag.isJsonPrimitive || !flag.asJsonPrimitive.isBoolean) throw WireException("Invalid diff flag")
            return flag.asBoolean
        }
        val diff = FrameCodec.string(value, "diff"); val status = FrameCodec.string(value, "status")
        if (diff.toByteArray().size > 65536 || status.toByteArray().size > 32768) throw WireException("Diff limit")
        val page = if (paged) {
            val raw = value["page"]?.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Missing diff page")
            require(raw.keySet() == setOf("digest", "offset", "next_offset", "total_bytes", "has_more"))
            fun number(field: String): Int {
                val n = raw[field]; require(n.isJsonPrimitive && n.asJsonPrimitive.isNumber && n.toString().matches(Regex("0|[1-9][0-9]*")))
                return requireNotNull(n.toString().toIntOrNull()?.takeIf { it in 0..1048576 })
            }
            val digest = FrameCodec.string(raw, "digest"); val offset = number("offset"); val next = number("next_offset"); val total = number("total_bytes")
            val more = raw["has_more"]; require(more.isJsonPrimitive && more.asJsonPrimitive.isBoolean)
            require(digest.matches(Regex("[0-9a-f]{64}")) && offset == (previous?.page?.nextOffset ?: 0)
                && diff.toByteArray(Charsets.UTF_8).size <= 8192 && next - offset == diff.toByteArray(Charsets.UTF_8).size
                && next <= total && more.asBoolean == (next < total) && (!more.asBoolean || next > offset))
            DiffPage(digest, offset, next, total, more.asBoolean)
        } else null
        if (previous != null) require(page?.digest == previous.page?.digest && page?.totalBytes == previous.page?.totalBytes
            && flag("changed") == previous.changed && flag("binary") == previous.binary && flag("truncated") == previous.truncated && status == previous.status)
        return RemoteDiff(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), subtabs.activeId,
            subtabs.revision, path, flag("changed"), flag("binary"), flag("truncated"), diff, status, page)
    }
    private fun consecutive(first: RemoteDiff, second: RemoteDiff): Boolean = first.page != null && second.page != null
        && first.copy(diff = second.diff, page = second.page) == second && first.page.digest == second.page.digest
        && first.page.totalBytes == second.page.totalBytes && first.page.hasMore && first.page.nextOffset == second.page.offset
    fun remember(pages: List<RemoteDiff>, current: RemoteDiff?, next: RemoteDiff): List<RemoteDiff> {
        if (current == null) { require(validDocument(next) && (next.page == null || next.page.offset == 0)); return emptyList() }
        require(validChain(pages, current) && validDocument(next) && consecutive(current, next))
        return (pages + current).takeLast(7)
    }
    fun previous(pages: List<RemoteDiff>, current: RemoteDiff): RemoteDiff? {
        require(validChain(pages, current))
        val previous = pages.lastOrNull() ?: return null
        return previous
    }
}
