package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger
import java.util.UUID

internal object TerminalCommandCodec {
    fun validateArguments(operation: String, args: JsonObject) {
        if (operation == "terminal_open") {
            require(args.keySet() == setOf("subtab_id", "subtab_revision")
                && FrameCodec.string(args, "subtab_id").matches(Regex("[0-9a-f]{64}")))
            unsigned(args, "subtab_revision"); return
        }
        val fields = setOf("terminal_id", "subtab_id", "subtab_revision") +
            if (operation == "terminal_input") setOf("text") else setOf("rows", "cols")
        require(operation in setOf("terminal_input", "terminal_resize") && args.keySet() == fields)
        for (field in listOf("terminal_id", "subtab_id")) require(FrameCodec.string(args, field).matches(Regex("[0-9a-f]{64}")))
        unsigned(args, "subtab_revision")
        if (operation == "terminal_input") {
            val text = FrameCodec.string(args, "text")
            require(text.isNotEmpty() && text.toByteArray(Charsets.UTF_8).size <= 4096 && !text.contains('\u0000'))
        } else for (field in listOf("rows", "cols")) require(unsigned(args, field) in BigInteger.ONE..BigInteger.valueOf(512))
    }
    fun open(projection: HostProjection): WorkspaceCommand {
        require(SessionCatalogCodec.readable(projection))
        val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.terminalOpenControl }
            ?: throw WireException("Terminal creation unavailable")
        val tabs = requireNotNull(SubtabProjectionCodec.current(projection))
        return WorkspaceCommand.restore(JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
            addProperty("required_owner", requireNotNull(projection.workspaceOwner))
            addProperty("expected_revision", BigInteger(requireNotNull(projection.workspaceRevision)))
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", "terminal_open")
                add("arguments", JsonObject().apply {
                    addProperty("subtab_id", tabs.activeId); addProperty("subtab_revision", BigInteger(tabs.revision))
                })
            })
        })
    }
    private fun unsigned(value: JsonObject, field: String): BigInteger {
        val number = value[field] ?: throw WireException("Missing terminal number")
        require(number.isJsonPrimitive && number.asJsonPrimitive.isNumber && number.toString().matches(Regex("0|[1-9][0-9]*"))
            && number.asBigInteger <= BigInteger("18446744073709551615"))
        return number.asBigInteger
    }
    fun create(projection: HostProjection, text: String? = null, rows: Int = 24, cols: Int = 80): WorkspaceCommand {
        val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.terminalControl }
            ?: throw WireException("Terminal control unavailable")
        val terminal = TerminalPageCodec.terminal(projection) ?: throw WireException("Terminal unavailable")
        require(terminal.running)
        val tabs = requireNotNull(SubtabProjectionCodec.current(projection))
        require(terminal.subtabId == tabs.activeId && terminal.subtabRevision == tabs.revision && tabs.projectId == project.id)
        val operation = if (text != null) "terminal_input" else "terminal_resize"
        return WorkspaceCommand.restore(JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
            addProperty("required_owner", requireNotNull(projection.workspaceOwner))
            addProperty("expected_revision", BigInteger(requireNotNull(projection.workspaceRevision)))
            add("payload", JsonObject().apply {
                addProperty("type", "request"); addProperty("operation", operation)
                add("arguments", JsonObject().apply {
                    addProperty("terminal_id", requireNotNull(terminal.terminalId)); addProperty("subtab_id", terminal.subtabId)
                    addProperty("subtab_revision", BigInteger(terminal.subtabRevision))
                    if (text != null) addProperty("text", text) else { addProperty("rows", rows); addProperty("cols", cols) }
                })
            })
        })
    }
    fun validateReceipt(request: JsonObject, result: JsonObject) {
        val payload = request["payload"].asJsonObject
        val args = payload["arguments"].asJsonObject
        if (FrameCodec.string(payload, "operation") == "terminal_open") {
            require(result.keySet() == setOf("request_id", "project_id", "session_id", "terminal_id", "subtab_id", "subtab_revision")
                && FrameCodec.string(result, "request_id") == FrameCodec.string(request, "id")
                && FrameCodec.string(result, "terminal_id").matches(Regex("[0-9a-f]{64}")))
            for (field in listOf("project_id", "session_id")) require(result[field] == request[field])
            for (field in listOf("subtab_id", "subtab_revision")) require(result[field] == args[field])
            unsigned(result, "subtab_revision"); return
        }
        val input = FrameCodec.string(payload, "operation") == "terminal_input"
        require(result.keySet() == setOf("request_id", "project_id", "session_id", "terminal_id", "subtab_id", "subtab_revision") +
            if (input) setOf("accepted_bytes") else setOf("rows", "cols"))
        require(FrameCodec.string(result, "request_id") == FrameCodec.string(request, "id"))
        for (field in listOf("project_id", "session_id")) require(result[field] == request[field])
        for (field in listOf("terminal_id", "subtab_id", "subtab_revision")) require(result[field] == args[field])
        unsigned(result, "subtab_revision")
        if (input) require(unsigned(result, "accepted_bytes") == BigInteger.valueOf(FrameCodec.string(args, "text").toByteArray(Charsets.UTF_8).size.toLong()))
        else for (field in listOf("rows", "cols")) require(unsigned(result, field) == unsigned(args, field))
    }
}
