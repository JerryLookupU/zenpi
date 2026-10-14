package zenpi.state

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import zenpi.transport.*
import java.math.BigInteger

/** Display and cursor are one record; a cursor alone must never skip unseen turns. */
internal object ProjectionCacheCodec {
    const val MAX_BYTES = 131_072
    const val MAX_PROMPT_STATUSES = 256
    fun encode(projection: HostProjection): JsonObject {
        val selected = projection.projects.singleOrNull { it.id == projection.selectedId }
            ?: throw WireException("Selected cache project required")
        if (projection.sessionId != selected.sessionId) throw WireException("Cache session mismatch")
        var remainingStatuses = MAX_PROMPT_STATUSES
        val statusPriority = listOf(selected) + projection.projects.filter { it.id != selected.id }
        val cachedStatuses = statusPriority.associate { project ->
            val statuses = project.promptStatuses.take(remainingStatuses)
            remainingStatuses -= statuses.size
            project.id to statuses
        }
        val snapshot = JsonObject().apply {
            addProperty("project_id", selected.id); addProperty("session_id", selected.sessionId)
            addProperty("phase", projection.phase); addProperty("cursor_scope", "session_journal")
            addProperty("journal_next_sequence", BigInteger(requireNotNull(projection.journalSequence)))
            addProperty("transcript_truncated", projection.truncated); addProperty("mutations_available", false)
            addProperty("event_cursor_scope", "transcript_events_v1")
            add("event_cursor", JsonObject().apply { addProperty("session_id", selected.sessionId)
                addProperty("next_sequence", BigInteger(requireNotNull(projection.eventSequence))) })
            add("transcript", JsonArray().apply { projection.turns.forEach { turn -> add(JsonObject().apply {
                addProperty("id", turn.id); addProperty("role", turn.role); addProperty("content", turn.content)
                if (turn.contentTruncated) addProperty("content_truncated", true)
                if (turn.toolCalls.isNotEmpty() || turn.toolCallsTruncated || turn.toolCallsOmitted) addProperty("tool_calls_omitted", true)
            }) } })
        }
        val listing = JsonObject().apply {
            addProperty("mutations_available", false); addProperty("active_project_id", selected.id)
            add("projects", JsonArray().apply { projection.projects.forEach { project -> add(JsonObject().apply {
                addProperty("id", project.id); addProperty("title", project.title); addProperty("session_id", project.sessionId)
                addProperty("workspace_control", false)
                add("prompt_statuses", JsonArray().apply { cachedStatuses.getValue(project.id).forEach { status -> add(JsonObject().apply {
                    addProperty("request_id", status.requestId); addProperty("project_id", status.projectId); addProperty("session_id", status.sessionId)
                    addProperty("state", if (status.state in setOf("accepted", "running")) "unknown_outcome" else status.state)
                    if(status.state=="failed")status.failureCode?.let{addProperty("failure_code",it)}
                }) } })
            }) } })
        }
        val subtabs = projection.subtabs?.let { state ->
            JsonObject().apply {
                addProperty("project_id", state.projectId); addProperty("subtab_revision", BigInteger(state.revision))
                addProperty("active_subtab_id", state.activeId)
                add("tabs", JsonArray().apply { state.tabs.forEach { tab -> add(JsonObject().apply {
                    addProperty("id", tab.id); addProperty("name", tab.name); addProperty("kind", tab.kind)
                    addProperty("concurrency", tab.concurrency)
                }) } })
            }
        }
        val encoded = JsonObject().apply {
            addProperty("version", 4); add("listing", listing); add("snapshot", snapshot)
            add("subtabs", subtabs ?: com.google.gson.JsonNull.INSTANCE)
        }
        val statusArrays = listing["projects"].asJsonArray.associate {
            FrameCodec.string(it.asJsonObject, "id") to it.asJsonObject["prompt_statuses"].asJsonArray
        }
        // Bound metadata before sacrificing any transcript display paired with its cursor.
        var encodedBytes = encoded.toString().toByteArray(Charsets.UTF_8).size
        for (project in statusPriority.asReversed()) {
            val statuses = statusArrays.getValue(project.id)
            while (encodedBytes > MAX_BYTES && statuses.size() > 0) {
                val commaBytes = if (statuses.size() > 1) 1 else 0
                val removed = statuses.remove(statuses.size() - 1)
                encodedBytes -= removed.toString().toByteArray(Charsets.UTF_8).size + commaBytes
            }
        }
        if (encodedBytes > MAX_BYTES) throw WireException("Cache byte limit")
        decode(encoded)
        return encoded
    }
    fun decode(value: JsonObject): HostProjection {
        val version = value["version"]?.toString()
        val fields = when (version) {
            "1" -> setOf("version", "listing", "snapshot")
            "2", "3", "4" -> setOf("version", "listing", "snapshot", "subtabs")
            else -> throw WireException("Invalid cache version")
        }
        if (value.keySet() != fields || value.toString().toByteArray().size > MAX_BYTES) throw WireException("Invalid cache fields or byte limit")
        val listing = value["listing"].asJsonObject
        if (listing.keySet() != setOf("mutations_available", "active_project_id", "projects")) throw WireException("Invalid cache listing fields")
        val projects = HostProjectionCodec.projects(listing)
        if (projects.any { it.workspaceControl || it.promptControl || it.approvalControl || it.workerControl || it.resourcesReloadControl || it.terminalControl || it.terminalOpenControl || it.sessionControl } || listing["projects"].asJsonArray.any {
            it.asJsonObject.keySet() != if (version in setOf("3", "4")) setOf("id", "title", "session_id", "workspace_control", "prompt_statuses")
                else setOf("id", "title", "session_id", "workspace_control")
        }) throw WireException("Cached workspace authority or fields")
        if (projects.any { project -> project.promptStatuses.any { it.state in setOf("accepted", "running") } }) throw WireException("Cached active prompt authority")
        val selected = HostProjectionCodec.select(listing, projects, null) ?: throw WireException("Missing cached selection")
        if (FrameCodec.string(listing, "active_project_id") != selected.id) throw WireException("Invalid cached selection")
        val snapshot = value["snapshot"].asJsonObject
        if (snapshot.keySet() != setOf("project_id", "session_id", "phase", "cursor_scope", "journal_next_sequence",
                "transcript_truncated", "mutations_available", "event_cursor_scope", "event_cursor", "transcript")
            || snapshot["event_cursor"].asJsonObject.keySet() != setOf("session_id", "next_sequence")
            || snapshot["transcript"].asJsonArray.any {
                val item = it.asJsonObject
                val turnFields = item.keySet() - "tool_calls_omitted"
                turnFields != setOf("id", "role", "content") && turnFields != setOf("id", "role", "content", "content_truncated")
                    || item.has("tool_calls_omitted") && (version !in setOf("3", "4") || FrameCodec.string(item, "role") != "assistant"
                        || !item["tool_calls_omitted"].isJsonPrimitive || !item["tool_calls_omitted"].asJsonPrimitive.isBoolean
                        || !item["tool_calls_omitted"].asBoolean)
            }) throw WireException("Invalid cache snapshot fields")
        val subtabs = if (version != "1" && !value["subtabs"].isJsonNull) {
            val display = value["subtabs"].asJsonObject
            if (display.keySet() != setOf("project_id", "subtab_revision", "active_subtab_id", "tabs")) throw WireException("Invalid cache subtab fields")
            SubtabProjectionCodec.decodeDisplay(display, selected.id)
        } else null
        // Legacy writers did not consistently mark withheld summaries; their cursor cannot prove completeness.
        val omitted = snapshot["transcript"].asJsonArray.filter {
            it.asJsonObject.has("tool_calls_omitted") || version != "4" && FrameCodec.string(it.asJsonObject, "role") == "assistant"
        }
            .map { FrameCodec.string(it.asJsonObject, "id") }.toSet()
        val decoded = HostProjectionCodec.snapshot(projects, selected, snapshot)
        return decoded.copy(subtabs = subtabs, turns = decoded.turns.map { it.copy(toolCallsOmitted = it.id in omitted) }).also {
            if (it.eventSequence == null) throw WireException("Missing cached cursor")
        }
    }
}
