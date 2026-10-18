package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

internal object SessionCommandCodec {
    fun mainSelected(projection: HostProjection?): Boolean = SubtabProjectionCodec.current(projection)?.let { tabs ->
        tabs.tabs.first().let { it.kind == "main" && it.id == tabs.activeId }
            && tabs.tabs.drop(1).none { it.kind == "main" }
    } == true

    fun validateName(name: String): String {
        require(name.isNotBlank() && name == name.trim() && name.toByteArray(Charsets.UTF_8).size <= 64
            && name.none { Character.isISOControl(it) || it == '/' || it == '\\' })
        return name
    }
    fun rename(projection: HostProjection, name: String): WorkspaceCommand {
        validateName(name)
        val value = newSession(projection).envelope
        value.getAsJsonObject("payload").apply { addProperty("operation", "session_rename"); add("arguments", JsonObject().apply {
            addProperty("name", name); addProperty("journal_revision", BigInteger(requireNotNull(projection.journalSequence)))
        }) }
        return WorkspaceCommand.restore(value)
    }
    fun renameReceipt(request: JsonObject, result: JsonObject) {
        require(result.keySet() == setOf("request_id", "project_id", "session_id", "name", "journal_revision"))
        require(result["request_id"] == request["id"] && result["project_id"] == request["project_id"] && result["session_id"] == request["session_id"])
        val args = request.getAsJsonObject("payload").getAsJsonObject("arguments")
        require(result["name"] == args["name"]); validateName(FrameCodec.string(result, "name"))
        val revision = result["journal_revision"]
        require(revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString().matches(Regex("[1-9][0-9]*"))
            && revision.asBigInteger <= BigInteger("18446744073709551615") && revision.asBigInteger > args["journal_revision"].asBigInteger)
    }
    fun resume(projection: HostProjection, catalog: SessionCatalog, target: String): WorkspaceCommand {
        require(SessionCatalogCodec.resumable(catalog, projection, target))
        val value = newSession(projection).envelope
        value.getAsJsonObject("payload").apply { addProperty("operation", "session_resume"); add("arguments", JsonObject().apply {
            addProperty("target_session_id", target); addProperty("snapshot_id", catalog.snapshotId)
        }) }
        return WorkspaceCommand.restore(value)
    }
    fun newSession(projection: HostProjection): WorkspaceCommand {
        require(SessionCatalogCodec.readable(projection))
        val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.sessionControl }
            ?: throw WireException("Session control unavailable")
        require(mainSelected(projection))
        return WorkspaceCommand.restore(JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
            addProperty("required_owner", requireNotNull(projection.workspaceOwner))
            addProperty("expected_revision", BigInteger(requireNotNull(projection.workspaceRevision)))
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "session_new"); add("arguments", JsonObject()) })
        })
    }
    fun receipt(request: JsonObject, result: JsonObject) {
        require(result.keySet() == setOf("request_id", "project_id", "old_session_id", "new_session_id", "required_owner", "workspace_revision", "active_project_id"))
        require(result["request_id"] == request["id"] && result["project_id"] == request["project_id"]
            && result["old_session_id"] == request["session_id"] && result["required_owner"] == request["required_owner"])
        val session = FrameCodec.string(result, "new_session_id")
        require(SessionCatalogCodec.validIdentity(session) && session != FrameCodec.string(request, "session_id"))
        if (FrameCodec.string(request["payload"].asJsonObject, "operation") == "session_resume")
            require(session == FrameCodec.string(request["payload"].asJsonObject["arguments"].asJsonObject, "target_session_id"))
        require(FrameCodec.string(result, "active_project_id").matches(Regex("[0-9a-f]{64}")))
        val revision = result["workspace_revision"]
        require(revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString().matches(Regex("[1-9][0-9]*"))
            && revision.asBigInteger <= BigInteger("18446744073709551615") && revision.asBigInteger == request["expected_revision"].asBigInteger + BigInteger.ONE)
    }
}
