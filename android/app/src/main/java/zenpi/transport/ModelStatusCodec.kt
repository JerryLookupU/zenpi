package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

data class RemoteModelStatus(val projectId: String, val sessionId: String, val model: String?, val reasoningEffort: String?,
    val registryBound: Boolean, val selectionRevision: String, val capabilities: Map<String, Boolean>?, val maxTokens: String, val reservedOutputTokens: String,
    val reasoningChoices: List<String?> = emptyList(), val catalog: RemoteModelCatalog? = null)
data class RemoteModelChoice(val id: String, val status: String, val selectable: Boolean)
data class RemoteModelCatalog(val snapshotId: String, val offset: Int, val nextOffset: Int?, val entries: List<RemoteModelChoice>)

internal object ModelStatusCodec {
    private val efforts = setOf("none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra")
    private val capabilityNames = setOf("text", "images", "files", "tools", "structured_output", "streaming", "reasoning")
    private fun validUnsigned(value: String): Boolean = value.matches(Regex("0|[1-9][0-9]*"))
        && value.toBigInteger() <= BigInteger("18446744073709551615")
    private fun validLabel(value: String): Boolean = value.isNotEmpty() && value.toByteArray(Charsets.UTF_8).size <= 128
        && value.none { Character.isISOControl(it) } && !value.startsWith('/') && !value.contains("://")
    private fun validStatus(status: RemoteModelStatus): Boolean {
        if (status.model?.let { validLabel(it) } == false || status.reasoningEffort?.let { it in efforts } == false
            || !validUnsigned(status.selectionRevision) || !validUnsigned(status.maxTokens) || !validUnsigned(status.reservedOutputTokens)
            || status.capabilities?.keys?.let { it == capabilityNames } == false
            || status.reasoningChoices.size > 9 || status.reasoningChoices.distinct().size != status.reasoningChoices.size
            || status.reasoningChoices.any { it != null && it !in efforts }) return false
        val page = status.catalog ?: return true
        val ids = page.entries.map { it.id }
        return page.snapshotId.matches(Regex("[0-9a-f]{64}")) && page.offset in 0..496 && page.offset % 16 == 0
            && page.entries.size <= 16 && (page.offset == 0 || page.entries.isNotEmpty())
            && (page.nextOffset == null || page.nextOffset == page.offset + 16 && page.nextOffset < 512 && page.entries.size == 16)
            && page.entries.all { validLabel(it.id) && it.status in setOf("active", "alpha", "beta", "deprecated") }
            && ids.distinct().size == ids.size && ids == ids.sortedWith(utf8Order)
    }
    fun validChain(pages: List<RemoteModelStatus>, current: RemoteModelStatus): Boolean = pages.size <= 7
        && validStatus(current) && pages.all(::validStatus)
        && pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
        && pages.lastOrNull()?.let { consecutive(it, current) } != false

    fun readable(projection: HostProjection): Boolean = projection.selectedId != null && projection.sessionId != null
        && projection.projects.singleOrNull { it.id == projection.selectedId }?.sessionId == projection.sessionId
    fun sameScope(status: RemoteModelStatus, projection: HostProjection?): Boolean = projection != null && readable(projection)
        && validStatus(status)
        && status.projectId == projection.selectedId && status.sessionId == projection.sessionId
        && projection.projects.singleOrNull { it.id == status.projectId && it.sessionId == status.sessionId } != null
    fun selectionCurrent(status: RemoteModelStatus, projection: HostProjection): Boolean = status.registryBound
        && sameScope(status, projection) && projection.modelStatus?.let {
            it.registryBound && sameScope(it, projection) && it.selectionRevision == status.selectionRevision
        } != false
    fun reasoningCurrent(status: RemoteModelStatus, projection: HostProjection, selected: String?): Boolean =
        selectionCurrent(status, projection) && selected in status.reasoningChoices
            && projection.modelStatus?.let { selected in it.reasoningChoices } != false
    private val utf8Order = Comparator<String> { a, b ->
        val left = a.toByteArray(Charsets.UTF_8); val right = b.toByteArray(Charsets.UTF_8)
        var result = 0
        for (index in 0 until minOf(left.size, right.size)) {
            result = (left[index].toInt() and 255).compareTo(right[index].toInt() and 255)
            if (result != 0) break
        }
        if (result != 0) result else left.size.compareTo(right.size)
    }
    private fun consecutive(first: RemoteModelStatus, second: RemoteModelStatus): Boolean = first.projectId == second.projectId
        && first.sessionId == second.sessionId && first.selectionRevision == second.selectionRevision
        && first.catalog != null && second.catalog != null && first.catalog.snapshotId == second.catalog.snapshotId
        && first.catalog.nextOffset != null && first.catalog.nextOffset == second.catalog.offset
        && first.catalog.entries.lastOrNull()?.let { last -> second.catalog.entries.firstOrNull()?.let {
            utf8Order.compare(last.id, it.id) < 0 } } == true
    fun remember(pages: List<RemoteModelStatus>, current: RemoteModelStatus?, next: RemoteModelStatus): List<RemoteModelStatus> {
        if (current == null) { require(validStatus(next) && (next.catalog == null || next.catalog.offset == 0)); return emptyList() }
        require(validChain(pages, current) && validStatus(next) && consecutive(current, next))
        require((pages + current).flatMap { requireNotNull(it.catalog).entries }.none { old -> next.catalog?.entries?.any { it.id == old.id } == true })
        return (pages + current).takeLast(7)
    }
    fun newest(displayed: RemoteModelStatus?, incoming: RemoteModelStatus?, projection: HostProjection?): RemoteModelStatus? {
        if (projection == null || !readable(projection)) return null
        fun scoped(value: RemoteModelStatus?) = value?.takeIf { sameScope(it, projection) }
        val old = scoped(displayed)
        val next = scoped(incoming) ?: return old
        if (old == null) return next
        val revision = BigInteger(old.selectionRevision).compareTo(BigInteger(next.selectionRevision))
        if (revision > 0) return old
        // Retain the user's catalog page only while it still belongs to the same snapshot.
        return if (revision == 0 && old.catalog != null && old.catalog.snapshotId == next.catalog?.snapshotId)
            next.copy(catalog = old.catalog) else next
    }
    fun retainsPage(previous: RemoteModelStatus?, next: RemoteModelStatus?): Boolean = previous != null && next != null
        && previous.projectId == next.projectId && previous.sessionId == next.sessionId
        && previous.selectionRevision == next.selectionRevision && previous.catalog != null && previous.catalog == next.catalog
    fun request(projection: HostProjection, previous: RemoteModelStatus? = null, offset: Int = 0): JsonObject {
        require(readable(projection))
        require(offset in 0..512 && offset % 16 == 0)
        if (previous != null) require(sameScope(previous, projection)
            && previous.catalog != null && previous.catalog.snapshotId.matches(Regex("[0-9a-f]{64}"))
            && (offset == previous.catalog.nextOffset || offset == previous.catalog.offset - 16))
        else require(offset == 0)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "model_status"); add("arguments", JsonObject().apply {
                if(previous!=null){addProperty("offset",offset);addProperty("snapshot_id",requireNotNull(previous.catalog).snapshotId)}
            }) })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, previous: RemoteModelStatus? = null, offset: Int = 0): RemoteModelStatus {
        request(projection, previous, offset)
        val fields = setOf("project_id", "session_id", "model", "reasoning_effort", "registry_bound", "selection_revision", "capabilities", "budget")
        if ((value.keySet() != fields && value.keySet() != fields + "reasoning_choices" && value.keySet() != fields + setOf("reasoning_choices","catalog"))
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId)
            throw WireException("Model status scope mismatch")
        val model = nullable(value, "model")
        val reasoning = nullable(value, "reasoning_effort")
        if (model != null && (model.isEmpty() || model.toByteArray().size > 128 || model.any { Character.isISOControl(it) })
            || reasoning != null && reasoning !in setOf("none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra")) throw WireException("Invalid model label")
        val capabilities = if (value["capabilities"].isJsonNull) null else value["capabilities"].takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid capabilities")
        if (capabilities != null && capabilities.keySet() != setOf("text", "images", "files", "tools", "structured_output", "streaming", "reasoning")) throw WireException("Invalid capabilities")
        val budget = value["budget"].takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid model budget")
        if (budget.keySet() != setOf("max_tokens", "reserved_output_tokens")) throw WireException("Invalid model budget")
        val choices = if (!value.has("reasoning_choices")) emptyList() else {
            val items = value["reasoning_choices"].takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Invalid reasoning choices")
            if (items.size() > 9) throw WireException("Invalid reasoning choices")
            items.map { item -> if (item.isJsonNull) null else {
                if (!item.isJsonPrimitive || !item.asJsonPrimitive.isString || item.asString !in setOf("none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra")) throw WireException("Invalid reasoning choices")
                item.asString
            } }.also { if (it.distinct().size != it.size) throw WireException("Duplicate reasoning choices") }
        }
        val catalog=if(value.has("catalog")) {
            val page=value["catalog"].takeIf{it.isJsonObject}?.asJsonObject ?: throw WireException("Invalid model catalog")
            if(page.keySet()!=setOf("snapshot_id","offset","next_offset","entries")) throw WireException("Invalid model catalog")
            val snapshot=FrameCodec.string(page,"snapshot_id")
            if(!snapshot.matches(Regex("[0-9a-f]{64}")) || unsigned(page,"offset")!=offset.toString()) throw WireException("Model catalog scope mismatch")
            if(previous!=null && (previous.catalog?.snapshotId!=snapshot || previous.selectionRevision!=unsigned(value,"selection_revision"))) throw WireException("Stale model catalog")
            val next=if(page["next_offset"].isJsonNull)null else unsigned(page,"next_offset").toIntOrNull() ?: throw WireException("Invalid model catalog offset")
            val entries=page["entries"].takeIf{it.isJsonArray}?.asJsonArray ?: throw WireException("Invalid model catalog")
            if(entries.size()>16 || offset !in 0..512 || offset%16!=0 || offset>0 && entries.size()==0
                || next!=null && (next!=offset+16 || next>512 || entries.size()!=16)) throw WireException("Invalid model catalog page")
            val modelChoices=entries.map{raw->
                val item=raw.takeIf{it.isJsonObject}?.asJsonObject ?: throw WireException("Invalid model choice")
                val id=FrameCodec.string(item,"id");val lifecycle=FrameCodec.string(item,"status")
                if(item.keySet()!=setOf("id","status","selectable") || id.isEmpty() || id.toByteArray().size>128 || id.any{Character.isISOControl(it)}
                    || id.startsWith("/") || id.contains("://") || lifecycle !in setOf("active","alpha","beta","deprecated")) throw WireException("Invalid model choice")
                RemoteModelChoice(id,lifecycle,flag(item,"selectable"))
            }
            val ids=modelChoices.map{it.id}
            if(ids.distinct().size!=ids.size || ids!=ids.sortedWith(utf8Order)) throw WireException("Invalid model catalog order")
            previous?.catalog?.let { old ->
                val before = if (offset > old.offset) old.entries else modelChoices
                val after = if (offset > old.offset) modelChoices else old.entries
                if (before.isNotEmpty() && after.isNotEmpty() && utf8Order.compare(before.last().id, after.first().id) >= 0)
                    throw WireException("Invalid model catalog continuation order")
            }
            RemoteModelCatalog(snapshot,offset,next,modelChoices)
        } else {if(previous!=null || offset!=0)throw WireException("Missing model catalog");null}
        return RemoteModelStatus(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), model, reasoning,
            flag(value, "registry_bound"), unsigned(value, "selection_revision"), capabilities?.keySet()?.associateWith { flag(capabilities, it) },
            unsigned(budget, "max_tokens"), unsigned(budget, "reserved_output_tokens"), choices, catalog)
            .also { if (!validStatus(it)) throw WireException("Invalid retained model status") }
    }
    private fun nullable(value: JsonObject, key: String): String? = if (value[key].isJsonNull) null else FrameCodec.string(value, key)
    private fun flag(value: JsonObject, key: String): Boolean {
        val raw = value[key]
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isBoolean) throw WireException("Invalid model flag")
        return raw.asBoolean
    }
    private fun unsigned(value: JsonObject, key: String): String {
        val raw = value[key]
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isNumber || !raw.toString().matches(Regex("0|[1-9][0-9]*"))
            || raw.asBigInteger > BigInteger("18446744073709551615")) throw WireException("Invalid model number")
        return raw.toString()
    }
}
