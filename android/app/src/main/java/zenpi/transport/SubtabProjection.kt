package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger

data class RemoteSubtab(val id: String, val name: String, val kind: String, val concurrency: Int)
data class RemoteSubtabs(val projectId: String, val revision: String, val activeId: String,
    val tabs: List<RemoteSubtab>, val control: Boolean, val directoryControl: Boolean = false,
    val worktreeControl: Boolean = false, val concurrencyControl: Boolean = false)

/** Only redacted view fields cross the network; paths and local execution never do. */
internal object SubtabProjectionCodec {
    private fun valid(tabs: RemoteSubtabs): Boolean = tabs.revision.matches(Regex("0|[1-9][0-9]*"))
        && tabs.revision.toBigInteger() <= BigInteger("18446744073709551615")
        && tabs.activeId.matches(Regex("[0-9a-f]{64}")) && tabs.tabs.size in 1..32
        && tabs.tabs.map { it.id }.distinct().size == tabs.tabs.size
        && tabs.tabs.singleOrNull { it.id == tabs.activeId } != null
        && tabs.tabs.withIndex().all { (index, tab) -> tab.id.matches(Regex("[0-9a-f]{64}"))
            && tab.kind in setOf("main", "worktree", "in_place") && (index == 0) == (tab.kind == "main")
            && tab.name.isNotBlank() && tab.name.toByteArray(Charsets.UTF_8).size <= 128
            && tab.name.none { Character.isISOControl(it) } && tab.concurrency in 1..64 }

    fun current(projection: HostProjection?): RemoteSubtabs? {
        if (!SessionCatalogCodec.readable(projection)) return null
        return projection?.subtabs?.takeIf { tabs ->
            tabs.projectId == projection.selectedId && valid(tabs)
        }
    }
    private fun hex(value: String) = value.also { if (!it.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid subtab identity") }
    private fun unsigned(value: com.google.gson.JsonElement): String {
        if (!value.isJsonPrimitive || !value.asJsonPrimitive.isNumber || !value.toString().matches(Regex("0|[1-9][0-9]*"))
            || value.asBigInteger > BigInteger("18446744073709551615")) throw WireException("Invalid subtab revision")
        return value.toString()
    }
    private fun flag(value: JsonObject, field: String): Boolean {
        val element = value[field] ?: throw WireException("Missing subtab capability")
        if (!element.isJsonPrimitive || !element.asJsonPrimitive.isBoolean) throw WireException("Invalid subtab capability")
        return element.asBoolean
    }
    fun decode(value: JsonObject, project: RemoteProject, workspace: JsonObject): RemoteSubtabs {
        if (value.keySet() != setOf("project_id", "session_id", "schema_version", "required_owner", "workspace_revision", "subtab_revision", "active_subtab_id", "tabs", "subtab_control", "directory_tab_control", "worktree_control", "concurrency_control")
            || value["schema_version"].toString() != "2" || FrameCodec.string(value, "project_id") != project.id
            || FrameCodec.string(value, "session_id") != project.sessionId
            || hex(FrameCodec.string(value, "required_owner")) != FrameCodec.string(workspace, "required_owner")) throw WireException("Subtab owner mismatch")
        if (unsigned(value["workspace_revision"]) != unsigned(workspace["workspace_revision"])) throw WireException("Subtab workspace changed")
        val display = decodeDisplay(value, project.id)
        project.terminal?.let { terminal ->
            if (terminal.projectId != project.id || terminal.sessionId != project.sessionId
                || terminal.subtabId != display.activeId || terminal.subtabRevision != display.revision)
                throw WireException("Terminal subtab selection changed")
        }
        val concurrencyControl = flag(value, "concurrency_control")
        val worktreeControl = flag(value, "worktree_control")
        val control = flag(value, "subtab_control")
        val directoryControl = flag(value, "directory_tab_control")
        if ((control || directoryControl || worktreeControl || concurrencyControl) && !project.workspaceControl) throw WireException("Unexpected subtab authority")
        return display.copy(control = control, directoryControl = directoryControl, worktreeControl = worktreeControl, concurrencyControl = concurrencyControl)
    }
    /** Shared view validation; this entry point never grants command authority. */
    fun decodeDisplay(value: JsonObject, projectId: String): RemoteSubtabs {
        if (FrameCodec.string(value, "project_id") != projectId) throw WireException("Subtab project mismatch")
        val revision = unsigned(value["subtab_revision"])
        val active = hex(FrameCodec.string(value, "active_subtab_id"))
        val items = value["tabs"]?.takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Missing subtabs")
        if (items.size() !in 1..32) throw WireException("Subtab capacity")
        val tabs = items.mapIndexed { index, item ->
            val tab = item.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid subtab")
            if (tab.keySet() != setOf("id", "name", "kind", "concurrency")) throw WireException("Invalid subtab fields")
            val kind = FrameCodec.string(tab, "kind")
            val name = FrameCodec.string(tab, "name")
            if (kind !in setOf("main", "worktree", "in_place") || (index == 0) != (kind == "main")
                || name.isBlank() || name.toByteArray().size > 128 || name.any { Character.isISOControl(it) }) throw WireException("Invalid subtab view")
            val concurrency = unsigned(tab["concurrency"]).toIntOrNull() ?: throw WireException("Invalid subtab concurrency")
            if (concurrency !in 1..64) throw WireException("Invalid subtab concurrency")
            RemoteSubtab(hex(FrameCodec.string(tab, "id")), name, kind, concurrency)
        }
        if (tabs.map { it.id }.distinct().size != tabs.size || tabs.none { it.id == active }) throw WireException("Invalid subtab selection")
        return RemoteSubtabs(projectId, revision, active, tabs, false, false)
            .also { if (!valid(it)) throw WireException("Invalid retained subtab view") }
    }
}
