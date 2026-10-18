package zenpi.transport

import com.google.gson.JsonArray
import com.google.gson.JsonElement

data class RemoteWorkerStatus(val requestId: String, val projectId: String, val sessionId: String,
    val subtabId: String, val state: String, val failureCode: String? = null,
    val outputText: String? = null, val outputTruncated: Boolean = false, val outputWithheld: Boolean = false,
    val streamText: String? = null, val streamTruncated: Boolean = false, val runtimeActive: Boolean = false) {
    val canCancel: Boolean get() = state in setOf("accepted", "running") || state == "unknown_outcome" && runtimeActive
}

internal object WorkerStatusCodec {
    fun validDisplay(status: RemoteWorkerStatus, project: String, session: String): Boolean =
        status.projectId == project && status.sessionId == session
            && status.requestId.isNotEmpty() && status.requestId.toByteArray(Charsets.UTF_8).size <= 128
            && status.requestId.none { Character.isISOControl(it) }
            && status.subtabId.matches(Regex("[0-9a-f]{64}")) && status.state in RemoteSchema.EXECUTION_STATES
            && (status.failureCode == null || status.state == "failed"
                && status.failureCode in RemoteSchema.ERROR_CODES - "session_changed")
            && (!status.runtimeActive || status.state in setOf("accepted", "running", "unknown_outcome"))
            && (status.outputText == null || status.state == "succeeded" && !status.outputWithheld
                && status.outputText.toByteArray(Charsets.UTF_8).size <= 4096 && !status.outputText.contains('\u0000'))
            && (!status.outputTruncated || status.outputText != null)
            && (!status.outputWithheld || status.state == "succeeded" && status.outputText == null && !status.outputTruncated)
            && (status.streamText == null || status.state in setOf("accepted", "running")
                && status.streamText.toByteArray(Charsets.UTF_8).size <= 4096 && !status.streamText.contains('\u0000'))
            && (!status.streamTruncated || status.streamText != null)

    fun validMetadata(status: RemoteWorkerStatus, project: String, session: String): Boolean =
        validDisplay(status, project, session)
            && status.outputText == null && !status.outputWithheld && !status.outputTruncated
            && status.streamText == null && !status.streamTruncated

    fun decode(value: JsonElement?, project: String, session: String): List<RemoteWorkerStatus> {
        if (value == null) return emptyList()
        if (!value.isJsonArray || value.asJsonArray.size() > 16) throw WireException("Worker status limit")
        return value.asJsonArray.map { element ->
            if (!element.isJsonObject) throw WireException("Invalid worker status")
            val item = element.asJsonObject
            val fields = setOf("request_id", "project_id", "session_id", "subtab_id", "state")
            val outputFields = setOf("output_text", "output_truncated", "output_withheld")
            val streamFields = setOf("stream_text", "stream_truncated")
            val plainFields = item.keySet() - outputFields - streamFields - "runtime_active"
            if (plainFields != fields && plainFields != fields + "failure_code") throw WireException("Invalid worker status fields")
            val subtab = FrameCodec.string(item, "subtab_id")
            if (!subtab.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid worker subtab")
            val plain = item.deepCopy().apply { remove("subtab_id"); remove("runtime_active"); outputFields.forEach { remove(it) } }
            val status = PromptStatusCodec.decode(JsonArray().apply { add(plain) }, project, session).single()
            val active = if (item.has("runtime_active")) item["runtime_active"].let {
                if (!it.isJsonPrimitive || !it.asJsonPrimitive.isBoolean || !it.asBoolean
                    || status.state !in setOf("accepted", "running", "unknown_outcome")) throw WireException("Invalid worker runtime evidence")
                true
            } else false
            val present = item.keySet().intersect(outputFields)
            if (present !in setOf(emptySet<String>(), setOf("output_text", "output_truncated"), setOf("output_withheld"))
                || present.isNotEmpty() && status.state != "succeeded") throw WireException("Invalid worker output state")
            fun flag(name: String): Boolean = item[name].let {
                if (!it.isJsonPrimitive || !it.asJsonPrimitive.isBoolean) throw WireException("Invalid worker output flag")
                it.asBoolean
            }
            val text = if (item.has("output_text")) FrameCodec.string(item, "output_text").also {
                if (it.toByteArray().size > 4096 || it.contains('\u0000')) throw WireException("Worker output limit")
            } else null
            val truncated = if (text != null) flag("output_truncated") else false
            val withheld = if (item.has("output_withheld")) flag("output_withheld").also {
                if (!it) throw WireException("Invalid withheld worker output")
            } else false
            RemoteWorkerStatus(status.requestId, project, session, subtab, status.state, status.failureCode, text, truncated, withheld, status.streamText, status.streamTruncated, active)
        }.also {
            if (it.map { status -> status.requestId }.distinct().size != it.size) throw WireException("Duplicate worker status")
            if (!it.all { status -> validDisplay(status, project, session) }) throw WireException("Invalid worker display metadata")
        }
    }
}
