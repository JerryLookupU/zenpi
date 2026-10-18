package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

data class RemoteResource(val name: String, val kind: String, val command: String, val description: String, val invocable: Boolean)
data class ResourceCatalog(val projectId: String, val sessionId: String, val generation: String,
    val entries: List<RemoteResource>, val truncated: Boolean, val snapshotId: String = "", val offset: Int = 0, val nextOffset: Int? = null)

internal object ResourceCatalogCodec {
    private fun validResource(resource: RemoteResource): Boolean = resource.name.matches(Regex("[A-Za-z0-9_.-]{1,128}"))
        && resource.kind in setOf("skill", "template")
        && resource.command == (if (resource.kind == "skill") "/skill:${resource.name}" else "/${resource.name}")
        && resource.description.toByteArray(Charsets.UTF_8).size <= 1024
        && resource.description.none { Character.isISOControl(it) && it != '\n' && it != '\t' }

    private fun validCatalog(catalog: ResourceCatalog): Boolean = catalog.generation.matches(Regex("0|[1-9][0-9]*"))
        && catalog.generation.toBigInteger() <= BigInteger("18446744073709551615")
        && catalog.snapshotId.matches(Regex("[0-9a-f]{64}")) && catalog.offset in 0..5120 && catalog.offset % 32 == 0
        && catalog.entries.size <= 32 && catalog.entries.all(::validResource)
        && catalog.entries.map { it.kind to it.name }.distinct().size == catalog.entries.size
        && (catalog.offset == 0 || catalog.entries.isNotEmpty()) && catalog.truncated == (catalog.nextOffset != null)
        && (catalog.nextOffset == null || catalog.entries.size == 32 && catalog.nextOffset == catalog.offset + 32
            && catalog.nextOffset <= 5120)

    fun validChain(pages: List<ResourceCatalog>, current: ResourceCatalog): Boolean {
        if (pages.size > 7 || !validCatalog(current) || !pages.all(::validCatalog)
            || !pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val ids = (pages + current).flatMap { it.entries }.map { it.kind to it.name }
        return ids.distinct().size == ids.size
    }

    fun readable(projection: HostProjection): Boolean = projection.selectedId != null && projection.sessionId != null
        && projection.projects.singleOrNull { it.id == projection.selectedId }?.sessionId == projection.sessionId
    fun sameScope(catalog: ResourceCatalog, projection: HostProjection?): Boolean = projection != null && readable(projection)
        && validCatalog(catalog)
        && catalog.projectId == projection.selectedId && catalog.sessionId == projection.sessionId
        && projection.projects.singleOrNull { it.id == catalog.projectId && it.sessionId == catalog.sessionId } != null
    private fun consecutive(first: ResourceCatalog, second: ResourceCatalog): Boolean = first.projectId == second.projectId
        && first.sessionId == second.sessionId && first.generation == second.generation && first.snapshotId == second.snapshotId
        && first.nextOffset != null && first.nextOffset == second.offset
    fun remember(pages: List<ResourceCatalog>, current: ResourceCatalog?, next: ResourceCatalog): List<ResourceCatalog> {
        if (current == null) { require(validCatalog(next) && next.offset == 0); return emptyList() }
        require(validChain(pages, current) && validCatalog(next) && consecutive(current, next))
        require((pages + current).flatMap { it.entries }.none { old -> next.entries.any { it.kind == old.kind && it.name == old.name } })
        return (pages + current).takeLast(7)
    }
    fun request(projection: HostProjection, previous: ResourceCatalog? = null, offset: Int = 0): JsonObject {
        require(readable(projection))
        require(previous == null && offset == 0 || previous != null && sameScope(previous, projection)
            && offset >= 0 && offset <= 5120 && offset % 32 == 0
            && (offset == previous.nextOffset || offset == previous.offset - 32))
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "resources_list")
                add("arguments", JsonObject().apply { if (previous != null) { addProperty("offset", offset); addProperty("snapshot_id", previous.snapshotId) } }) })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, previous: ResourceCatalog? = null, requestedOffset: Int = 0): ResourceCatalog {
        request(projection, previous, requestedOffset)
        if (value.keySet() != setOf("project_id", "session_id", "generation", "entries", "truncated", "snapshot_id", "offset", "next_offset")
            || FrameCodec.string(value, "project_id") != projection.selectedId
            || FrameCodec.string(value, "session_id") != projection.sessionId) throw WireException("Resource scope mismatch")
        val generation = value["generation"] ?: throw WireException("Missing resource generation")
        if (!generation.isJsonPrimitive || !generation.asJsonPrimitive.isNumber
            || !generation.toString().matches(Regex("0|[1-9][0-9]*"))
            || generation.asBigInteger > BigInteger("18446744073709551615")) throw WireException("Invalid resource generation")
        val snapshotId = FrameCodec.string(value, "snapshot_id")
        val offset = offset(value, "offset")
        val nextOffset = if (value["next_offset"].isJsonNull) null else offset(value, "next_offset")
        if (!snapshotId.matches(Regex("[0-9a-f]{64}")) || offset != requestedOffset
            || requestedOffset > 0 && previous == null
            || previous != null && (previous.snapshotId != snapshotId || previous.generation != generation.toString()
                || previous.projectId != projection.selectedId || previous.sessionId != projection.sessionId)) throw WireException("Resource snapshot changed")
        val raw = value["entries"]?.takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Invalid resources")
        if (raw.size() > 32) throw WireException("Resource limit")
        val entries = raw.map { element ->
            val item = element.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid resource")
            if (item.keySet() != setOf("name", "kind", "command", "description", "invocable")) throw WireException("Invalid resource fields")
            val name = FrameCodec.string(item, "name"); val kind = FrameCodec.string(item, "kind")
            val command = FrameCodec.string(item, "command"); val description = FrameCodec.string(item, "description")
            RemoteResource(name, kind, command, description, flag(item, "invocable"))
                .also { if (!validResource(it)) throw WireException("Invalid resource metadata") }
        }
        if (entries.map { it.kind to it.name }.distinct().size != entries.size) throw WireException("Duplicate resource")
        if (previous?.entries?.any { old -> entries.any { it.kind == old.kind && it.name == old.name } } == true)
            throw WireException("Repeated resource page entry")
        val more = flag(value, "truncated")
        if (more != (nextOffset != null) || more && (entries.size != 32 || nextOffset != offset + 32)
            || offset > 0 && entries.isEmpty()) throw WireException("Invalid resource continuation")
        return ResourceCatalog(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), generation.toString(), entries, more, snapshotId, offset, nextOffset)
            .also { if (!validCatalog(it)) throw WireException("Invalid retained resource catalog") }
    }
    fun insert(resource: RemoteResource, draft: String): String {
        require(validResource(resource) && resource.invocable && !draft.contains('\u0000')
            && !draft.trimStart().startsWith('/') && !draft.trimStart().startsWith('!'))
        return (resource.command + " " + draft).also { require(it.toByteArray(Charsets.UTF_8).size <= 65536) }
    }
    private fun flag(value: JsonObject, key: String): Boolean {
        val raw = value[key] ?: throw WireException("Missing resource flag")
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isBoolean) throw WireException("Invalid resource flag")
        return raw.asBoolean
    }
    private fun offset(value: JsonObject, key: String): Int {
        val raw = value[key] ?: throw WireException("Missing resource offset")
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isNumber || !raw.toString().matches(Regex("0|[1-9][0-9]*"))
            || raw.asBigInteger > BigInteger.valueOf(5120) || raw.asBigInteger.mod(BigInteger.valueOf(32)) != BigInteger.ZERO)
            throw WireException("Invalid resource offset")
        return raw.asInt
    }
}
