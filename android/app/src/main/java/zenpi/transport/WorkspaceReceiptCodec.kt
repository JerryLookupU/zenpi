package zenpi.transport

import com.google.gson.JsonObject
import java.math.BigInteger

internal object WorkspaceReceiptCodec {
    fun validate(command: WorkspaceCommand, response: JsonObject): JsonObject {
        val request = command.envelope
        if (response.keySet().any { it !in setOf("schema_version", "channel", "id", "session_id", "project_id", "code", "execution_state", "payload") })
            throw WireException("Workspace receipt envelope mismatch")
        if (FrameCodec.string(response, "execution_state") != "failed" && response["code"]?.isJsonNull == false)
            throw WireException("Unexpected workspace receipt code")
        val operation = FrameCodec.string(request["payload"].asJsonObject, "operation")
        val admission = operation in setOf("prompt", "steer", "cancel", "approve")
        if (FrameCodec.string(response, "id") != FrameCodec.string(request, "id")
            || FrameCodec.string(response, "channel") != "control"
            || FrameCodec.string(response["payload"].asJsonObject, "type") != "response"
            || FrameCodec.string(response, "execution_state") != if (admission) "accepted" else "succeeded")
            throw WireException("Workspace receipt mismatch")
        for (field in listOf("project_id", "session_id")) {
            val routed = response[field]
            if (routed != null && !routed.isJsonNull) {
                val requestValue = request[field]
                if (requestValue != null && !requestValue.isJsonNull
                    && FrameCodec.string(response, field) != FrameCodec.string(request, field))
                    throw WireException("Workspace receipt $field mismatch")
            }
        }
        val result = response["payload"].asJsonObject["result"]
        if (result == null || !result.isJsonObject) throw WireException("Invalid workspace receipt")
        val value = result.asJsonObject
        if (operation == "session_rename") {
            SessionCommandCodec.renameReceipt(request, value)
        } else if (operation in setOf("session_new", "session_resume")) {
            SessionCommandCodec.receipt(request, value)
        } else if (operation in setOf("terminal_open", "terminal_input", "terminal_resize")) {
            TerminalCommandCodec.validateReceipt(request, value)
        } else if (operation == "resources_reload") {
            if (value.keySet() != setOf("request_id", "project_id", "session_id", "generation")
                || FrameCodec.string(value, "request_id") != FrameCodec.string(request, "id")
                || FrameCodec.string(value, "project_id") != FrameCodec.string(request, "project_id")
                || FrameCodec.string(value, "session_id") != FrameCodec.string(request, "session_id"))
                throw WireException("Resource reload receipt mismatch")
            val generation = value["generation"]
            if (!generation.isJsonPrimitive || !generation.asJsonPrimitive.isNumber
                || !generation.toString().matches(Regex("[1-9][0-9]*"))
                || generation.asBigInteger > BigInteger("18446744073709551615")
                || generation.asBigInteger <= request["payload"].asJsonObject["arguments"].asJsonObject["generation"].asBigInteger)
                throw WireException("Resource generation receipt mismatch")
        } else if (operation in setOf("model_select", "reasoning_select")) {
            val args = request["payload"].asJsonObject["arguments"].asJsonObject
            val field = if (operation == "model_select") "model" else "reasoning_effort"
            if (value.keySet() != setOf("request_id", "project_id", "session_id", field, "selection_revision")
                || FrameCodec.string(value, "request_id") != FrameCodec.string(request, "id")
                || FrameCodec.string(value, "project_id") != FrameCodec.string(request, "project_id")
                || FrameCodec.string(value, "session_id") != FrameCodec.string(request, "session_id")
                || value[field] != args[field]) throw WireException("Model receipt mismatch")
            val revision = value["selection_revision"]
            if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("[1-9][0-9]*"))
                || revision.asBigInteger > BigInteger("18446744073709551615")
                || revision.asBigInteger <= args["selection_revision"].asBigInteger) throw WireException("Model revision receipt mismatch")
        } else if (admission) {
            val workerPrompt = operation == "prompt" && request["payload"].asJsonObject["arguments"].asJsonObject.has("subtab_id")
            val fields = if (operation == "approve") setOf("request_id", "target_request_id", "session_id", "project_id", "digest", "decision")
                else setOf("request_id", if (operation in setOf("cancel", "steer")) "target_request_id" else "job_id", "session_id", "project_id") + if (workerPrompt) setOf("subtab_id") else emptySet()
            if (value.keySet() != fields || FrameCodec.string(value, "request_id") != FrameCodec.string(request, "id")
                || FrameCodec.string(value, "project_id") != FrameCodec.string(request, "project_id")
                || FrameCodec.string(value, "session_id") != FrameCodec.string(request, "session_id")) throw WireException("Admission receipt mismatch")
            if (operation == "prompt") {
                if (workerPrompt && FrameCodec.string(value, "subtab_id") != FrameCodec.string(request["payload"].asJsonObject["arguments"].asJsonObject, "subtab_id")) throw WireException("Worker receipt mismatch")
                val job = FrameCodec.string(value, "job_id")
                if (job.isBlank() || job.toByteArray().size > 128 || job.any { Character.isISOControl(it) }) throw WireException("Invalid job receipt")
            } else {
                val arguments = request["payload"].asJsonObject["arguments"].asJsonObject
                if (FrameCodec.string(value, "target_request_id") != FrameCodec.string(arguments, "request_id")) throw WireException("Target receipt mismatch")
                if (operation == "approve") for (field in listOf("digest", "decision")) {
                    if (FrameCodec.string(value, field) != FrameCodec.string(arguments, field)) throw WireException("Approval receipt mismatch")
                }
            }
        } else {
            val subtab = operation.startsWith("subtab_") || operation == "set_concurrency"
            val fields = setOf("workspace_revision", "active_project_id", "required_owner") +
                if (subtab) setOf("project_id", "active_subtab_id", "subtab_revision") else emptySet()
            if (value.keySet() != fields || FrameCodec.string(value, "required_owner") != FrameCodec.string(request, "required_owner"))
                throw WireException("Workspace owner receipt mismatch")
            fun hex(field: String) = FrameCodec.string(value, field).also {
                if (!it.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid receipt identity")
            }
            fun unsigned(objectValue: JsonObject, field: String): BigInteger {
                val number = objectValue[field]
                if (number == null || !number.isJsonPrimitive || !number.asJsonPrimitive.isNumber
                    || !number.toString().matches(Regex("0|[1-9][0-9]*")) || number.asBigInteger > BigInteger("18446744073709551615"))
                    throw WireException("Invalid receipt revision")
                return number.asBigInteger
            }
            val activeProject = hex("active_project_id")
            if ((subtab || operation == "workspace_select") && activeProject != FrameCodec.string(request, "project_id")
                || operation == "workspace_close" && activeProject == FrameCodec.string(request, "project_id"))
                throw WireException("Workspace selection receipt mismatch")
            if (unsigned(value, "workspace_revision") != unsigned(request, "expected_revision") + BigInteger.ONE)
                throw WireException("Workspace revision receipt mismatch")
            if (subtab) {
                if (hex("project_id") != FrameCodec.string(request, "project_id")) throw WireException("Subtab project receipt mismatch")
                val activeTab = hex("active_subtab_id")
                val arguments = request["payload"].asJsonObject["arguments"].asJsonObject
                if (operation == "subtab_select" && activeTab != FrameCodec.string(arguments, "subtab_id")
                    || operation == "subtab_close" && activeTab == FrameCodec.string(arguments, "subtab_id"))
                    throw WireException("Subtab selection receipt mismatch")
                if (unsigned(value, "subtab_revision") != unsigned(arguments, "subtab_revision") + BigInteger.ONE)
                    throw WireException("Subtab revision receipt mismatch")
            }
        }
        return value
    }
}
