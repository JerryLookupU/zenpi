package zenpi.transport

import com.google.gson.JsonArray
import com.google.gson.JsonObject
import kotlinx.coroutines.*
import zenpi.pairing.AddressHint
import zenpi.security.DeviceIdentity
import zenpi.security.PairedHost
import java.net.InetSocketAddress
import java.net.Socket
import java.security.SecureRandom
import java.util.UUID
import javax.net.ssl.SSLContext
import javax.net.ssl.SSLSocket

data class RemoteProject(val id: String, val title: String, val sessionId: String, val workspaceControl: Boolean = false,
    val promptControl: Boolean = false, val promptStatuses: List<RemotePromptStatus> = emptyList(),
    val pendingApprovals: List<RemoteApproval> = emptyList(), val approvalControl: Boolean = false,
    val diffAvailable: Boolean = false, val desktopStreamText: String? = null, val terminal: RemoteTerminal? = null,
    val modelControl: Boolean = false, val reasoningControl: Boolean = false,
    val workerControl: Boolean = false, val workerStatuses: List<RemoteWorkerStatus> = emptyList(),
    val resourcesReloadControl: Boolean = false, val terminalControl: Boolean = false, val sessionControl: Boolean = false,
    val sessionName: String? = null, val terminalOpenControl: Boolean = false)
data class RemoteToolCall(val id: String, val name: String, val argumentsPreview: String, val argumentsTruncated: Boolean)
data class RemoteTurn(val id: String, val role: String, val content: String, val contentTruncated: Boolean = false,
    val toolCalls: List<RemoteToolCall> = emptyList(), val toolCallsTruncated: Boolean = false, val toolCallsOmitted: Boolean = false)
data class HostProjection(val projects: List<RemoteProject>, val selectedId: String?, val sessionId: String?,
    val phase: String?, val turns: List<RemoteTurn>, val journalSequence: String?, val truncated: Boolean,
    val eventSequence: String? = null, val hasMoreEvents: Boolean = false, val replayedEventCount: Int = 0,
    val workspaceOwner: String? = null, val workspaceRevision: String? = null,
    val subtabs: RemoteSubtabs? = null, val modelStatus: RemoteModelStatus? = null,
    val snapshotReplaced: Boolean = false)

class WorkspaceCommand private constructor(private val value: JsonObject) {
    val envelope: JsonObject get() = value.deepCopy()
    companion object {
        fun restore(value: JsonObject): WorkspaceCommand {
            val checked = FrameCodec.decode(value.toString().toByteArray())
            if (checked.keySet() != setOf("schema_version", "channel", "id", "project_id", "session_id", "required_owner", "expected_revision", "payload")
                || FrameCodec.string(checked, "channel") != "control") throw WireException("Invalid saved workspace request")
            FrameCodec.string(checked, "project_id"); FrameCodec.string(checked, "session_id")
            if (!FrameCodec.string(checked, "required_owner").matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid workspace owner")
            val payload = checked["payload"].asJsonObject
            if (FrameCodec.string(payload, "type") != "request") throw WireException("Invalid saved workspace operation")
            val operation = FrameCodec.string(payload, "operation")
            val arguments = payload["arguments"].asJsonObject
            when (operation) {
                "session_rename" -> {
                    require(arguments.keySet() == setOf("name", "journal_revision"))
                    SessionCommandCodec.validateName(FrameCodec.string(arguments, "name"))
                    val revision = arguments["journal_revision"]
                    require(revision.isJsonPrimitive && revision.asJsonPrimitive.isNumber && revision.toString().matches(Regex("0|[1-9][0-9]*"))
                        && revision.asBigInteger <= java.math.BigInteger("18446744073709551615"))
                }
                "session_new" -> if (arguments.keySet().isNotEmpty()) throw WireException("Invalid new session arguments")
                "session_resume" -> {
                    require(arguments.keySet() == setOf("target_session_id", "snapshot_id")
                        && SessionCatalogCodec.validIdentity(FrameCodec.string(arguments, "target_session_id"))
                        && FrameCodec.string(arguments, "snapshot_id").matches(Regex("[0-9a-f]{64}")))
                }
                "terminal_open", "terminal_input", "terminal_resize" -> TerminalCommandCodec.validateArguments(operation, arguments)
                "resources_reload" -> {
                    val generation = arguments["generation"]
                    if (arguments.keySet() != setOf("generation", "snapshot_id") || generation == null
                        || !generation.isJsonPrimitive || !generation.asJsonPrimitive.isNumber
                        || !generation.toString().matches(Regex("0|[1-9][0-9]*"))
                        || generation.asBigInteger > java.math.BigInteger("18446744073709551615")
                        || !FrameCodec.string(arguments, "snapshot_id").matches(Regex("[0-9a-f]{64}")))
                        throw WireException("Invalid resource reload")
                }
                "reasoning_select" -> {
                    if (arguments.keySet() != setOf("reasoning_effort", "selection_revision")) throw WireException("Invalid reasoning selection")
                    val effort = arguments["reasoning_effort"]
                    val revision = arguments["selection_revision"]
                    if (!effort.isJsonNull && (!effort.isJsonPrimitive || !effort.asJsonPrimitive.isString
                            || effort.asString !in setOf("none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra"))
                        || !revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
                        || revision.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid reasoning selection")
                }
                "model_select" -> {
                    if (arguments.keySet() != setOf("model", "selection_revision")) throw WireException("Invalid model selection")
                    val model = FrameCodec.string(arguments, "model")
                    val revision = arguments["selection_revision"]
                    if (model.isEmpty() || model.toByteArray().size > 128 || model.any { Character.isISOControl(it) }
                        || model.startsWith("/") || model.contains("://")
                        || !revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
                        || revision.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid model selection")
                }
                "steer" -> if (arguments.keySet() != setOf("request_id", "text")
                    || FrameCodec.string(arguments, "request_id").isBlank() || FrameCodec.string(arguments, "request_id").toByteArray().size > 128
                    || FrameCodec.string(arguments, "request_id").any { Character.isISOControl(it) }
                    || FrameCodec.string(arguments, "text").isBlank() || FrameCodec.string(arguments, "text").toByteArray().size > 65536
                    || FrameCodec.string(arguments, "text").contains('\u0000') || FrameCodec.string(arguments, "text").trimStart().startsWith("!")) throw WireException("Invalid steering input")
                "prompt" -> {
                    if (arguments.keySet() !in setOf(setOf("text"), setOf("text", "subtab_id", "subtab_revision"))
                        || FrameCodec.string(arguments, "text").isBlank() || FrameCodec.string(arguments, "text").toByteArray().size > 65536
                        || FrameCodec.string(arguments, "text").contains('\u0000')
                        || FrameCodec.string(arguments, "text").trimStart().startsWith("!")) throw WireException("Invalid prompt")
                    if (arguments.has("subtab_id")) {
                        val revision = arguments["subtab_revision"]
                        if (!FrameCodec.string(arguments, "subtab_id").matches(Regex("[0-9a-f]{64}"))
                            || !revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
                            || revision.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid worker scope")
                    }
                }
                "cancel" -> if (arguments.keySet() != setOf("request_id") || FrameCodec.string(arguments, "request_id").isBlank()
                    || FrameCodec.string(arguments, "request_id").toByteArray().size > 128
                    || FrameCodec.string(arguments, "request_id").any { Character.isISOControl(it) }) throw WireException("Invalid cancellation target")
                "approve" -> if (arguments.keySet() !in setOf(setOf("request_id", "digest", "decision"), setOf("request_id", "digest", "decision", "worker_session_id"))
                    || FrameCodec.string(arguments, "request_id").isBlank() || FrameCodec.string(arguments, "request_id").toByteArray().size > 128
                    || FrameCodec.string(arguments, "request_id").any { Character.isISOControl(it) }
                    || arguments.has("worker_session_id") && FrameCodec.string(arguments, "worker_session_id").let {
                        it.isBlank() || it.toByteArray().size > 128 || it.any { char -> Character.isISOControl(char) }
                    }
                    || !FrameCodec.string(arguments, "digest").matches(Regex("[0-9a-f]{64}"))
                    || FrameCodec.string(arguments, "decision") !in setOf("allow", "deny")) throw WireException("Invalid approval decision")
                "workspace_open" -> if (arguments.keySet() != setOf("cwd") || FrameCodec.string(arguments, "cwd").isBlank()
                    || FrameCodec.string(arguments, "cwd").toByteArray().size > 4096) throw WireException("Invalid workspace directory")
                "workspace_select", "workspace_close" -> if (arguments.size() != 0) throw WireException("Invalid workspace arguments")
                "set_concurrency" -> {
                    if (arguments.keySet() != setOf("subtab_id", "subtab_revision", "concurrency")
                        || !FrameCodec.string(arguments, "subtab_id").matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid concurrency target")
                    val revision = arguments["subtab_revision"]
                    val count = arguments["concurrency"]
                    if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
                        || revision.asBigInteger > java.math.BigInteger("18446744073709551615")
                        || !count.isJsonPrimitive || !count.asJsonPrimitive.isNumber || !count.toString().matches(Regex("[1-9][0-9]*"))
                        || count.asBigInteger !in java.math.BigInteger.ONE..java.math.BigInteger.valueOf(64)) throw WireException("Invalid concurrency value")
                }
                "subtab_select", "subtab_rename", "subtab_move", "subtab_close", "subtab_add" -> {
                    val keys = when(operation) { "subtab_rename" -> setOf("subtab_id", "subtab_revision", "name")
                        "subtab_move" -> setOf("subtab_id", "subtab_revision", "before_id")
                        "subtab_add" -> if (FrameCodec.string(arguments, "kind") == "worktree") setOf("kind", "name", "subtab_revision") else setOf("kind", "name", "cwd", "subtab_revision")
                        else -> setOf("subtab_id", "subtab_revision") }
                    if (arguments.keySet() != keys || (operation != "subtab_add" && !FrameCodec.string(arguments, "subtab_id").matches(Regex("[0-9a-f]{64}")))) throw WireException("Invalid subtab arguments")
                    val revision = arguments["subtab_revision"]
                    if (!revision.isJsonPrimitive || !revision.asJsonPrimitive.isNumber || !revision.toString().matches(Regex("0|[1-9][0-9]*"))
                        || revision.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid subtab revision")
                    if (operation == "subtab_rename" || operation == "subtab_add") {
                        val name = FrameCodec.string(arguments, "name")
                        if (name.isEmpty() || name.trim() != name || name.toByteArray().size > 128 || name.any { Character.isISOControl(it) }) throw WireException("Invalid subtab name")
                    }
                    if (operation == "subtab_add" && FrameCodec.string(arguments, "kind") == "worktree") {
                        if (!FrameCodec.string(arguments, "name").matches(Regex("[A-Za-z0-9_][A-Za-z0-9_-]{0,127}"))) throw WireException("Invalid worktree name")
                    } else if (operation == "subtab_add" && (FrameCodec.string(arguments, "kind") != "in_place"
                        || FrameCodec.string(arguments, "cwd").isBlank() || FrameCodec.string(arguments, "cwd").toByteArray().size > 4096)) throw WireException("Invalid directory tab")
                    if (operation == "subtab_move" && !arguments["before_id"].isJsonNull
                        && !FrameCodec.string(arguments, "before_id").matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid subtab target")
                }
                else -> throw WireException("Invalid saved workspace operation")
            }
            return WorkspaceCommand(checked)
        }
        fun create(projection: HostProjection, projectId: String, operation: String, cwd: String? = null): WorkspaceCommand {
            val project = projection.projects.singleOrNull { it.id == projectId }?.takeIf { it.workspaceControl }
                ?: throw WireException("Workspace control unavailable")
            require(operation in setOf("workspace_open", "workspace_select", "workspace_close"))
            val owner = requireNotNull(projection.workspaceOwner)
            val revision = requireNotNull(projection.workspaceRevision)
            val arguments = JsonObject()
            if (operation == "workspace_open") { require(!cwd.isNullOrBlank()); arguments.addProperty("cwd", cwd) }
            else require(cwd == null)
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", owner); addProperty("expected_revision", java.math.BigInteger(revision))
                add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", operation); add("arguments", arguments) })
            }
            return restore(value)
        }
        fun subtab(projection: HostProjection, operation: String, tabId: String, text: String? = null): WorkspaceCommand {
            val subtabs = requireNotNull(SubtabProjectionCodec.current(projection))
            val target = subtabs.tabs.singleOrNull { it.id == tabId }
            require(subtabs.control && subtabs.projectId == projection.selectedId && target != null)
            require(operation in setOf("subtab_select", "subtab_rename", "subtab_move", "subtab_close"))
            if (operation == "subtab_move" && text != null) require(subtabs.tabs.singleOrNull { it.id == text } != null)
            if (operation == "subtab_close") require(when (target.kind) {
                "in_place" -> subtabs.directoryControl
                "worktree" -> subtabs.worktreeControl
                else -> false
            })
            val value = create(projection, subtabs.projectId, "workspace_select").envelope
            val arguments = JsonObject().apply {
                addProperty("subtab_id", tabId); addProperty("subtab_revision", java.math.BigInteger(subtabs.revision))
                if (operation == "subtab_rename") addProperty("name", requireNotNull(text))
                if (operation == "subtab_move") add("before_id", text?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE)
            }
            value["payload"].asJsonObject.apply { addProperty("operation", operation); add("arguments", arguments) }
            return restore(value)
        }
        fun directoryTab(projection: HostProjection, name: String, cwd: String): WorkspaceCommand {
            val subtabs = requireNotNull(SubtabProjectionCodec.current(projection))
            require(subtabs.directoryControl && subtabs.projectId == projection.selectedId)
            val value = create(projection, subtabs.projectId, "workspace_select").envelope
            value["payload"].asJsonObject.apply {
                addProperty("operation", "subtab_add")
                add("arguments", JsonObject().apply {
                    addProperty("kind", "in_place"); addProperty("name", name); addProperty("cwd", cwd)
                    addProperty("subtab_revision", java.math.BigInteger(subtabs.revision))
                })
            }
            return restore(value)
        }
        fun worktreeTab(projection: HostProjection, name: String): WorkspaceCommand {
            val subtabs = requireNotNull(SubtabProjectionCodec.current(projection))
            require(subtabs.worktreeControl && subtabs.projectId == projection.selectedId)
            val value = create(projection, subtabs.projectId, "workspace_select").envelope
            value["payload"].asJsonObject.apply {
                addProperty("operation", "subtab_add")
                add("arguments", JsonObject().apply {
                    addProperty("kind", "worktree"); addProperty("name", name)
                    addProperty("subtab_revision", java.math.BigInteger(subtabs.revision))
                })
            }
            return restore(value)
        }
        fun prompt(projection: HostProjection, text: String): WorkspaceCommand {
            require(SessionCatalogCodec.readable(projection))
            val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.promptControl }
                ?: throw WireException("Prompt control unavailable")
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner))
                addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply {
                    addProperty("type", "request"); addProperty("operation", "prompt")
                    add("arguments", JsonObject().apply { addProperty("text", text) })
                })
            }
            return restore(value)
        }
        fun reloadResources(projection: HostProjection, catalog: ResourceCatalog): WorkspaceCommand {
            require(ResourceCatalogCodec.sameScope(catalog, projection))
            val project = projection.projects.singleOrNull {
                it.id == projection.selectedId && it.sessionId == projection.sessionId && it.resourcesReloadControl
            } ?: throw WireException("Resource reload unavailable")
            require(catalog.projectId == project.id && catalog.sessionId == project.sessionId
                && catalog.snapshotId.matches(Regex("[0-9a-f]{64}")))
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner))
                addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply {
                    addProperty("type", "request"); addProperty("operation", "resources_reload")
                    add("arguments", JsonObject().apply {
                        addProperty("generation", java.math.BigInteger(catalog.generation)); addProperty("snapshot_id", catalog.snapshotId)
                    })
                })
            }
            return restore(value)
        }
        fun workerPrompt(projection: HostProjection, text: String): WorkspaceCommand {
            require(SessionCatalogCodec.readable(projection))
            val project = projection.projects.singleOrNull { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.workerControl }
                ?: throw WireException("Worker control unavailable")
            val tabs = requireNotNull(SubtabProjectionCodec.current(projection))
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner))
                addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply {
                    addProperty("type", "request"); addProperty("operation", "prompt")
                    add("arguments", JsonObject().apply {
                        addProperty("text", text); addProperty("subtab_id", tabs.activeId)
                        addProperty("subtab_revision", java.math.BigInteger(tabs.revision))
                    })
                })
            }
            return restore(value)
        }
        fun concurrency(projection: HostProjection, tabId: String, count: Int): WorkspaceCommand {
            val tabs = requireNotNull(SubtabProjectionCodec.current(projection))
            require(tabs.concurrencyControl && tabs.projectId == projection.selectedId && tabs.tabs.singleOrNull { it.id == tabId } != null && count in 1..64)
            val value = create(projection, tabs.projectId, "workspace_select").envelope
            value["payload"].asJsonObject.apply {
                addProperty("operation", "set_concurrency")
                add("arguments", JsonObject().apply {
                    addProperty("subtab_id", tabId); addProperty("subtab_revision", java.math.BigInteger(tabs.revision)); addProperty("concurrency", count)
                })
            }
            return restore(value)
        }
        fun cancelWorker(projection: HostProjection, requestId: String): WorkspaceCommand {
            require(SessionCatalogCodec.readable(projection))
            val project = projection.projects.single { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.workerControl }
            return cancelWorker(projection, project.workerStatuses.single { it.requestId == requestId })
        }
        fun cancelWorker(projection: HostProjection, target: RemoteWorkerStatus): WorkspaceCommand {
            require(SessionCatalogCodec.readable(projection))
            val project = projection.projects.single { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.workerControl }
            require(WorkerStatusCodec.validDisplay(target, project.id, project.sessionId) && target.canCancel)
            val summary = project.workerStatuses.filter { it.requestId == target.requestId }
            require(summary.isEmpty() || summary.singleOrNull() == target)
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner))
                addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply {
                    addProperty("type", "request"); addProperty("operation", "cancel")
                    add("arguments", JsonObject().apply { addProperty("request_id", target.requestId) })
                })
            }
            return restore(value)
        }
        fun selectModel(projection: HostProjection, status: RemoteModelStatus, model: String): WorkspaceCommand {
            return selection(projection, status, "model_select", model)
        }
        fun selectReasoning(projection: HostProjection, status: RemoteModelStatus, effort: String?): WorkspaceCommand {
            require(status.reasoningChoices.contains(effort))
            return selection(projection, status, "reasoning_select", effort)
        }
        private fun selection(projection: HostProjection, status: RemoteModelStatus, operation: String, selected: String?): WorkspaceCommand {
            require(ModelStatusCodec.selectionCurrent(status, projection))
            if (operation == "reasoning_select") require(ModelStatusCodec.reasoningCurrent(status, projection, selected))
            val project = projection.projects.single { it.id == projection.selectedId && if (operation == "model_select") it.modelControl else it.reasoningControl }
            require(project.sessionId == status.sessionId)
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner))
                addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply {
                    addProperty("type", "request"); addProperty("operation", operation)
                    add("arguments", JsonObject().apply {
                        addProperty(if (operation == "model_select") "model" else "reasoning_effort", selected)
                        addProperty("selection_revision", java.math.BigInteger(status.selectionRevision))
                    })
                })
            }
            return restore(value)
        }
        fun cancelPrompt(projection: HostProjection, requestId: String): WorkspaceCommand {
            require(PromptStatusCodec.active(projection, requestId) != null)
            val value = prompt(projection, "cancel").envelope
            value["payload"].asJsonObject.apply {
                addProperty("operation", "cancel"); add("arguments", JsonObject().apply { addProperty("request_id", requestId) })
            }
            return restore(value)
        }
        fun steer(projection: HostProjection, requestId: String, text: String): WorkspaceCommand {
            require(SteeringInput.valid(text))
            require(PromptStatusCodec.active(projection, requestId) != null)
            val value = prompt(projection, text).envelope
            value["payload"].asJsonObject.apply {
                addProperty("operation", "steer")
                add("arguments", JsonObject().apply { addProperty("request_id", requestId); addProperty("text", text) })
            }
            return restore(value)
        }
        fun approve(projection: HostProjection, target: RemoteApproval, decision: String): WorkspaceCommand {
            require(ApprovalProjectionCodec.current(projection, target))
            val project = projection.projects.single { it.id == projection.selectedId && it.sessionId == projection.sessionId && it.approvalControl }
            require(project.pendingApprovals.any { it == target } && target.projectId == project.id && target.sessionId == projection.sessionId)
            val value = JsonObject().apply {
                addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
                addProperty("project_id", project.id); addProperty("session_id", project.sessionId)
                addProperty("required_owner", requireNotNull(projection.workspaceOwner)); addProperty("expected_revision", java.math.BigInteger(requireNotNull(projection.workspaceRevision)))
                add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "approve")
                    add("arguments", JsonObject().apply {
                        addProperty("request_id", target.requestId); addProperty("digest", target.digest); addProperty("decision", decision)
                        target.workerSessionId?.let { addProperty("worker_session_id", it) }
                    }) })
            }
            return restore(value)
        }
    }
}

internal object HostProjectionCodec {
    private fun identifier(value: String): String = value.also {
        if (it.isEmpty() || it.toByteArray().size > 128 || it.any { char -> Character.isISOControl(char) })
            throw WireException("Invalid projection identifier")
    }
    private fun boolean(value: JsonObject, field: String): Boolean {
        val element = value[field] ?: throw WireException("Missing $field")
        if (!element.isJsonPrimitive || !element.asJsonPrimitive.isBoolean) throw WireException("Invalid $field")
        return element.asBoolean
    }
    fun projects(listing: JsonObject): List<RemoteProject> {
        if (boolean(listing, "mutations_available")) throw WireException("Unsupported mutation capability")
        val items = listing["projects"]?.takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Missing projects")
        if (items.size() > 64) throw WireException("Workspace limit")
        val projects = items.map { value ->
            val item = value.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid project")
            val title = FrameCodec.string(item, "title")
            if (title.toByteArray().size > 4096 || title.any { Character.isISOControl(it) }) throw WireException("Invalid project title")
            RemoteProject(identifier(FrameCodec.string(item, "id")), title, identifier(FrameCodec.string(item, "session_id")),
                if (item.has("workspace_control")) boolean(item, "workspace_control") else false,
                if (item.has("prompt_control")) boolean(item, "prompt_control") else false,
                PromptStatusCodec.decode(item["prompt_statuses"], FrameCodec.string(item, "id"), FrameCodec.string(item, "session_id")),
                ApprovalProjectionCodec.decode(item["pending_approvals"], FrameCodec.string(item, "id"), FrameCodec.string(item, "session_id")),
                if (item.has("approval_control")) boolean(item, "approval_control") else false,
                if (item.has("diff_available")) boolean(item, "diff_available") else false,
                DesktopStreamCodec.decode(item["desktop_stream_text"]),
                TerminalProjectionCodec.decode(item["desktop_terminal"], FrameCodec.string(item, "id"), FrameCodec.string(item, "session_id")),
                if (item.has("model_control")) boolean(item, "model_control") else false,
                if (item.has("reasoning_control")) boolean(item, "reasoning_control") else false,
                if (item.has("worker_control")) boolean(item, "worker_control") else false,
                WorkerStatusCodec.decode(item["worker_statuses"], FrameCodec.string(item, "id"), FrameCodec.string(item, "session_id")),
                if (item.has("resources_reload_control")) boolean(item, "resources_reload_control") else false,
                if (item.has("terminal_control")) boolean(item, "terminal_control") else false,
                if (item.has("session_control")) boolean(item, "session_control") else false,
                item["session_name"]?.takeUnless { it.isJsonNull }?.let { SessionCommandCodec.validateName(FrameCodec.string(item, "session_name")) },
                if (item.has("terminal_open_control")) boolean(item, "terminal_open_control") else false)
        }
        if (projects.map { it.id }.distinct().size != projects.size) throw WireException("Duplicate project")
        return projects
    }
    fun select(listing: JsonObject, projects: List<RemoteProject>, selectedId: String?): RemoteProject? {
        val active = listing["active_project_id"] ?: throw WireException("Missing active project")
        val activeId = if (active.isJsonNull) null else identifier(FrameCodec.string(listing, "active_project_id"))
        if (activeId != null && projects.none { it.id == activeId }) throw WireException("Unknown active project")
        if (selectedId != null) return projects.find { it.id == selectedId } ?: throw SelectedProjectUnavailableException(selectedId)
        return projects.find { it.id == activeId } ?: projects.firstOrNull()
    }
    fun workspace(projection: HostProjection, listing: JsonObject): HostProjection {
        if (!listing.has("required_owner") && !listing.has("workspace_revision"))
            return projection.copy(workspaceOwner = null, workspaceRevision = null)
        val owner = FrameCodec.string(listing, "required_owner")
        if (!owner.matches(Regex("[0-9a-f]{64}"))) throw WireException("Invalid workspace owner")
        return projection.copy(workspaceOwner = owner, workspaceRevision = unsigned(listing["workspace_revision"]))
    }
    fun snapshot(projects: List<RemoteProject>, selected: RemoteProject, snapshot: JsonObject): HostProjection {
        if (FrameCodec.string(snapshot, "project_id") != selected.id || FrameCodec.string(snapshot, "session_id") != selected.sessionId
            || FrameCodec.string(snapshot, "cursor_scope") != "session_journal") throw WireException("Session owner or cursor mismatch")
        if (boolean(snapshot, "mutations_available")) throw WireException("Unsupported mutation capability")
        val phase = FrameCodec.string(snapshot, "phase")
        if (phase !in setOf("idle", "running", "closed")) throw WireException("Unknown host phase")
        val sequence = snapshot["journal_next_sequence"] ?: throw WireException("Missing sequence")
        if (!sequence.isJsonPrimitive || !sequence.asJsonPrimitive.isNumber || !sequence.toString().matches(Regex("0|[1-9][0-9]*"))
            || sequence.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid journal sequence")
        val turns = snapshot["transcript"]?.takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Missing transcript")
        if (turns.size() > 64) throw WireException("Transcript limit")
        val parsed = turns.map { value ->
            val turn = value.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid turn")
            val role = FrameCodec.string(turn, "role")
            if (role !in setOf("system", "user", "assistant", "tool")) throw WireException("Unknown turn role")
            decodeTurn(turn, role)
        }
        if (parsed.map { it.id }.distinct().size != parsed.size) throw WireException("Duplicate turn")
        if (snapshot.has("event_cursor") != snapshot.has("event_cursor_scope")) throw WireException("Incomplete event cursor")
        val cursor = snapshot["event_cursor"]?.takeIf { it.isJsonObject }?.asJsonObject
            ?: if (snapshot.has("event_cursor")) throw WireException("Invalid event cursor") else null
        val eventSequence = cursor?.let {
            if (it.keySet() != setOf("session_id", "next_sequence")
                || FrameCodec.string(snapshot, "event_cursor_scope") != "transcript_events_v1" || FrameCodec.string(it, "session_id") != selected.sessionId)
                throw WireException("Event cursor mismatch")
            unsigned(it["next_sequence"])
        }
        eventSequence?.let { requireCursorWithinJournal(it, sequence.toString()) }
        val retained = displayTail(parsed)
        return HostProjection(projects, selected.id, selected.sessionId, phase, retained, sequence.toString(),
            boolean(snapshot, "transcript_truncated") || retained.size != parsed.size || retained.any { it.contentTruncated }, eventSequence)
    }

    private fun decodeTurn(value: JsonObject, role: String): RemoteTurn {
        val truncated = if (value.has("content_truncated")) {
            if (!boolean(value, "content_truncated")) throw WireException("Invalid turn truncation")
            true
        } else false
        val content = FrameCodec.string(value, "content")
        // Accept the prior bounded host/cache format too; current hosts send
        // at most 12 KiB per turn, while legacy pages had a 96 KiB budget.
        if (content.toByteArray(Charsets.UTF_8).size > 96 * 1024) throw WireException("Turn content limit")
        val calls = ToolCallProjectionCodec.decode(value, role)
        return RemoteTurn(identifier(FrameCodec.string(value, "id")), role, content, truncated,
            calls, value["tool_calls_truncated"]?.asBoolean ?: false)
    }

    // Bound JSON-encoded display size too: control bytes expand when escaped.
    private fun displayTail(turns: List<RemoteTurn>): List<RemoteTurn> {
        var bytes = 0
        val kept = mutableListOf<RemoteTurn>()
        fun size(turn: RemoteTurn): Int = JsonObject().apply {
            addProperty("id", turn.id); addProperty("role", turn.role); addProperty("content", turn.content)
            if (turn.contentTruncated) addProperty("content_truncated", true)
            if (turn.toolCalls.isNotEmpty() || turn.toolCallsTruncated) {
                addProperty("tool_calls_truncated", turn.toolCallsTruncated)
                add("tool_calls", JsonArray().apply { turn.toolCalls.forEach { call -> add(JsonObject().apply {
                    addProperty("id", call.id); addProperty("name", call.name)
                    addProperty("arguments_preview", call.argumentsPreview); addProperty("arguments_truncated", call.argumentsTruncated)
                }) } })
            }
        }.toString().toByteArray(Charsets.UTF_8).size
        for (original in turns.asReversed().take(64)) {
            var turn = original
            var encoded = size(turn)
            // Keep an oversized legacy turn's identity and scalar-safe prefix
            // so its full-message paging control remains reachable.
            if (kept.isEmpty() && encoded > 96 * 1024) {
                fun prefix(length: Int): String {
                    val end = if (length in 1 until original.content.length && Character.isHighSurrogate(original.content[length - 1])) length - 1 else length
                    return original.content.substring(0, end)
                }
                var low = 0
                var high = original.content.length
                while (low < high) {
                    val middle = low + (high - low + 1) / 2
                    if (size(original.copy(content = prefix(middle), contentTruncated = true)) <= 96 * 1024) low = middle
                    else high = middle - 1
                }
                turn = original.copy(content = prefix(low), contentTruncated = true)
                encoded = size(turn)
            }
            if (bytes + encoded > 96 * 1024) break
            bytes += encoded; kept.add(turn)
        }
        return kept.asReversed()
    }

    private fun unsigned(value: com.google.gson.JsonElement?): String {
        if (value == null || !value.isJsonPrimitive || !value.asJsonPrimitive.isNumber || !value.toString().matches(Regex("0|[1-9][0-9]*"))
            || value.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid event sequence")
        return value.toString()
    }

    private fun requireCursorWithinJournal(event: String, journal: String) {
        // Each durable turn consumes a journal record; private records consume no event sequence.
        if (java.math.BigInteger(event) > java.math.BigInteger(journal))
            throw WireException("Event cursor exceeds journal sequence")
    }

    fun replay(previous: HostProjection, projects: List<RemoteProject>, selected: RemoteProject, frames: List<JsonObject>, result: JsonObject): HostProjection {
        if (previous.sessionId != selected.sessionId || previous.selectedId != selected.id) throw WireException("Replay owner mismatch")
        val start = java.math.BigInteger(previous.eventSequence ?: throw WireException("Missing previous cursor"))
        if (FrameCodec.string(result, "cursor_scope") != "transcript_events_v1") throw WireException("Replay cursor scope")
        val cursor = result["cursor"].asJsonObject
        if (cursor.keySet() != setOf("session_id", "next_sequence")
            || FrameCodec.string(cursor, "session_id") != selected.sessionId) throw WireException("Replay session mismatch")
        val end = unsigned(cursor["next_sequence"])
        val journal = unsigned(result["journal_next_sequence"])
        requireCursorWithinJournal(end, journal)
        if (boolean(result, "snapshot_required")) {
            if (boolean(result, "has_more")) throw WireException("Snapshot cannot have pending tail")
            if (frames.size != 1 || FrameCodec.string(frames[0]["payload"].asJsonObject, "type") != "snapshot") throw WireException("Snapshot required")
            val payload = frames[0]["payload"].asJsonObject
            if (payload["cursor"] != cursor || FrameCodec.string(frames[0], "project_id") != selected.id
                || FrameCodec.string(frames[0], "session_id") != selected.sessionId) throw WireException("Snapshot cursor mismatch")
            return snapshot(projects, selected, payload["state"].asJsonObject).also {
                if (it.eventSequence != end || it.journalSequence != journal
                    || it.phase != FrameCodec.string(result, "phase")) throw WireException("Snapshot terminal mismatch")
            }.copy(snapshotReplaced = true)
        }
        if (frames.size > 128 || previous.sessionId != selected.sessionId || previous.selectedId != selected.id) throw WireException("Replay owner mismatch")
        var next = start
        val turns = previous.turns.toMutableList()
        for (frame in frames) {
            val payload = frame["payload"].asJsonObject
            if (FrameCodec.string(payload, "type") != "event" || FrameCodec.string(payload, "name") != "transcript_turn"
                || FrameCodec.string(frame, "session_id") != selected.sessionId || FrameCodec.string(frame, "project_id") != selected.id
                || java.math.BigInteger(unsigned(frame["sequence"])) != next) throw WireException("Replay gap or owner mismatch")
            val data = payload["data"].asJsonObject
            val id = identifier(FrameCodec.string(data, "id")); val role = FrameCodec.string(data, "role")
            if (FrameCodec.string(frame, "turn_id") != id || role !in setOf("system", "user", "assistant", "tool") || turns.any { it.id == id })
                throw WireException("Replay turn mismatch")
            turns.add(decodeTurn(data, role))
            next += java.math.BigInteger.ONE
        }
        if (next.toString() != end) throw WireException("Replay terminal cursor mismatch")
        val phase = FrameCodec.string(result, "phase")
        if (phase !in setOf("idle", "running", "closed")) throw WireException("Replay phase mismatch")
        val hasMore = boolean(result, "has_more")
        if (hasMore && frames.isEmpty()) throw WireException("Replay made no progress")
        val retained = displayTail(turns)
        return previous.copy(projects = projects, phase = phase, turns = retained,
            truncated = previous.truncated || retained.size != turns.size || retained.any { it.contentTruncated }, journalSequence = journal, eventSequence = end,
            hasMoreEvents = hasMore, snapshotReplaced = false,
            replayedEventCount = (previous.replayedEventCount.toLong() + frames.size).coerceAtMost(Int.MAX_VALUE.toLong()).toInt())
    }
}

/** An event tail cannot restore invocation summaries deliberately omitted from cache. */
internal object TranscriptRestorePolicy {
    private fun sameCursor(previous: HostProjection?, selected: RemoteProject): Boolean {
        if (previous == null || previous.sessionId != selected.sessionId || previous.selectedId != selected.id
            || previous.projects.size > 64 || previous.projects.map { it.id }.distinct().size != previous.projects.size
            || previous.projects.singleOrNull { it.id == selected.id }?.sessionId != selected.sessionId) return false
        fun unsigned(value: String?): java.math.BigInteger? = value?.takeIf { it.matches(Regex("0|[1-9][0-9]*")) }
            ?.toBigInteger()?.takeIf { it <= java.math.BigInteger("18446744073709551615") }
        val cursor = unsigned(previous.eventSequence) ?: return false
        val journal = unsigned(previous.journalSequence) ?: return false
        return cursor <= journal
    }
    fun replay(previous: HostProjection?, selected: RemoteProject): Boolean = previous != null
        && sameCursor(previous, selected)
        && previous.turns.none { it.toolCallsOmitted }

    /**
     * While a turn holds the transcript, an unchanged cursor can still take the workspace listing
     * (pending approvals included). A subscription catches the transcript up later; a one-shot
     * read only takes this when the caller opts in with [busyListing] (ZE-544 background reads).
     */
    fun busyView(previous: HostProjection?, selected: RemoteProject, projects: List<RemoteProject>, listing: JsonObject,
        subtabs: RemoteSubtabs?, subscribe: Boolean, subscriptionAvailable: Boolean, mutation: Boolean,
        error: Exception, busyListing: Boolean = false): HostProjection {
        val follows = if (subscribe) subscriptionAvailable else busyListing
        if (!sameCursor(previous, selected) || !follows || mutation
            || error !is HostRefusedException || error.code != "busy") throw error
        val next = HostProjectionCodec.workspace(requireNotNull(previous).copy(projects = projects, hasMoreEvents = false), listing)
            .copy(subtabs = subtabs)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(previous, next)
        return next
    }
}

/** Bounded authenticated snapshot or durable transcript replay, never mutation retry. */
class HostClient(private val bindSocket: (Socket, String) -> Unit = { _, _ -> }) {
    /** One pinned TLS handshake that only learns the host's key id; no frame is sent. */
    suspend fun hostId(host: PairedHost): String = withContext(Dispatchers.IO) {
        val (hint, connected) = HintDialer.dial(host.hints) { hint, timeout ->
            val attempt = Socket()
            try {
                bindSocket(attempt, hint.host)
                attempt.connect(InetSocketAddress(hint.host, hint.port), timeout)
                attempt
            } catch (error: Throwable) { attempt.close(); throw error }
        }
        connected.use { raw ->
            val trust = PinnedTrustManager(host.fingerprint)
            val context = SSLContext.getInstance("TLSv1.3")
            context.init(null, arrayOf(trust), SecureRandom())
            (context.socketFactory.createSocket(raw, "zenpi.local", hint.port, true) as SSLSocket).use { socket ->
                socket.enabledProtocols = arrayOf("TLSv1.3"); socket.soTimeout = 5000
                socket.sslParameters = socket.sslParameters.apply { applicationProtocols = arrayOf("zenpi/1") }
                socket.startHandshake()
                requireNotNull(trust.verified) { "Host certificate unverified" }.hostId
            }
        }
    }
    suspend fun modelStatus(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: RemoteModelStatus? = null, offset: Int = 0): RemoteModelStatus {
        var result: RemoteModelStatus? = null
        inspect(host, projection.selectedId, identity, previous = projection, modelRequest = ModelStatusCodec.request(projection,previous,offset),
            onModel = { result = ModelStatusCodec.decode(it, projection,previous,offset) })
        return result ?: throw WireException("Missing model status")
    }
    suspend fun resources(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: ResourceCatalog? = null, offset: Int = 0): ResourceCatalog {
        var result: ResourceCatalog? = null
        inspect(host, projection.selectedId, identity, previous = projection, resourcesRequest = ResourceCatalogCodec.request(projection, previous, offset),
            onResources = { result = ResourceCatalogCodec.decode(it, projection, previous, offset) })
        return result ?: throw WireException("Missing resource catalog")
    }
    suspend fun files(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, path: String, previous: WorkspaceFiles? = null, offset: Int = 0): WorkspaceFiles {
        var result: WorkspaceFiles? = null
        try {
            inspect(host, projection.selectedId, identity, previous = projection, filesRequest = WorkspaceFilesCodec.request(projection, path,previous,offset),
                onFiles = { result = WorkspaceFilesCodec.decode(it, projection, path,previous,offset) })
        } catch(error:HostRefusedException) {
            if(error.code!="invalid_request" || previous!=null || offset!=0)throw error
            inspect(host, projection.selectedId, identity, previous = projection, filesRequest = WorkspaceFilesCodec.request(projection,path,paged=false),
                onFiles={result=WorkspaceFilesCodec.decode(it,projection,path)})
        }
        return result ?: throw WireException("Missing file listing")
    }
    suspend fun workerStatuses(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: WorkerStatusPage?, offset: Int): WorkerStatusPage {
        var result: WorkerStatusPage? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = WorkerStatusPageCodec.request(projection, previous, offset),
            onTranscript = { result = WorkerStatusPageCodec.decode(it, projection, previous, offset) })
        return result ?: throw WireException("Missing task list")
    }
    suspend fun transcript(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, turnId: String, previous: TranscriptPage? = null, worker: Boolean = false, historyTurn: RemoteTurn? = null, toolCall: RemoteToolCall? = null,
        journalRevision: String? = null): TranscriptPage {
        var result: TranscriptPage? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = TranscriptPageCodec.request(projection, turnId, previous, worker, historyTurn, toolCall, journalRevision),
            onTranscript = { result = TranscriptPageCodec.decode(it, projection, turnId, previous, worker, historyTurn, toolCall, journalRevision) })
        return result ?: throw WireException("Missing transcript page")
    }
    suspend fun terminalPage(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: TerminalPage?): TerminalPage {
        var result: TerminalPage? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = TerminalPageCodec.request(projection, previous),
            onTranscript = { result = TerminalPageCodec.decode(it, projection, previous) })
        return result ?: throw WireException("Missing terminal page")
    }
    suspend fun sessionCatalog(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: SessionCatalog?, offset: Int, query: String? = previous?.query): SessionCatalog {
        var result: SessionCatalog? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = SessionCatalogCodec.request(projection, previous, offset, query),
            onTranscript = { result = SessionCatalogCodec.decode(it, projection, previous, offset, query) })
        return result ?: throw WireException("Missing session catalog")
    }
    suspend fun workerHistory(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, requestId: String, previous: WorkerHistoryPage? = null): WorkerHistoryPage {
        var result: WorkerHistoryPage? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = WorkerHistoryCodec.request(projection, requestId, previous),
            onTranscript = { result = WorkerHistoryCodec.decode(it, projection, requestId, previous) })
        return result ?: throw WireException("Missing worker history")
    }
    suspend fun sessionHistory(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, previous: SessionHistoryPage?): SessionHistoryPage {
        var result: SessionHistoryPage? = null
        inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = SessionHistoryCodec.request(projection, previous),
            onTranscript = { result = SessionHistoryCodec.decode(it, projection, previous) })
        return result ?: throw WireException("Missing session history")
    }
    suspend fun diff(host: PairedHost, identity: DeviceIdentity, projection: HostProjection, path: String, previous: RemoteDiff? = null): RemoteDiff {
        var result: RemoteDiff? = null
        try {
            inspect(host, projection.selectedId, identity, previous = projection, transcriptRequest = DiffProjectionCodec.request(projection, path, previous, paged = true),
                onTranscript = { result = DiffProjectionCodec.decode(it, projection, path, previous, paged = true) })
        } catch (error: HostRefusedException) {
            if (error.code != "invalid_request" || previous != null) throw error
            inspect(host, projection.selectedId, identity, previous = projection, diffPath = path, onDiff = { result = it })
        }
        return result ?: throw WireException("Missing diff response")
    }
    suspend fun revokeSelf(host: PairedHost, identity: DeviceIdentity, requestId: String) {
        require(java.util.UUID.fromString(requestId).toString() == requestId)
        inspect(host, identity = identity, revokeRequestId = requestId)
    }
    suspend fun checkRevoke(host: PairedHost, identity: DeviceIdentity, requestId: String): Boolean {
        var committed = false
        inspect(host, identity = identity, revokeStatusId = requestId, onRevokeStatus = { committed = it })
        return committed
    }
    suspend fun inspect(host: PairedHost, selectedId: String? = null, identity: DeviceIdentity, previous: HostProjection? = null,
        workspaceCommand: WorkspaceCommand? = null, revokeRequestId: String? = null, revokeStatusId: String? = null,
        onRevokeStatus: (Boolean) -> Unit = {},
        onWorkspaceConfirmed: suspend () -> Unit = {}, acknowledgePrevious: Boolean = false,
        recoverSessionChanges: Boolean = false, busyListing: Boolean = false,
        diffPath: String? = null, onDiff: (RemoteDiff) -> Unit = {}, subscribe: Boolean = false,
        transcriptRequest: JsonObject? = null, onTranscript: (JsonObject) -> Unit = {},
        filesRequest: JsonObject? = null, onFiles: (JsonObject) -> Unit = {},
        resourcesRequest: JsonObject? = null, onResources: (JsonObject) -> Unit = {},
        modelRequest: JsonObject? = null, onModel: (JsonObject) -> Unit = {},
        onProjection: suspend (HostProjection) -> Unit = {}): HostProjection {
        if (modelRequest != null && (resourcesRequest != null || filesRequest != null || transcriptRequest != null || diffPath != null || workspaceCommand != null || revokeRequestId != null || revokeStatusId != null || subscribe)) throw WireException("Conflicting model read")
        if (resourcesRequest != null && (filesRequest != null || transcriptRequest != null || diffPath != null || workspaceCommand != null || revokeRequestId != null || revokeStatusId != null || subscribe)) throw WireException("Conflicting resource read")
        if (transcriptRequest != null && (diffPath != null || workspaceCommand != null || revokeRequestId != null || revokeStatusId != null || subscribe)) throw WireException("Conflicting transcript read")
        if (filesRequest != null && (transcriptRequest != null || diffPath != null || workspaceCommand != null || revokeRequestId != null || revokeStatusId != null || subscribe)) throw WireException("Conflicting file read")
        if (diffPath != null && (workspaceCommand != null || revokeRequestId != null || revokeStatusId != null))
            throw WireException("Conflicting read request")
        val deviceId = withContext(Dispatchers.IO) {
            AuthenticationProof.deviceId(host.deviceId) { identity.deviceId }
        }
        val active = SocketLease()
        return coroutineScope {
            val closer = launch(start = CoroutineStart.UNDISPATCHED) { try { awaitCancellation() } finally { active.close() } }
            val scope = this
            var deadline = launch { delay(15_000); active.close() }
            // Each subscription page gets the same 15 s watchdog as the opening exchange.
            fun rearm() { deadline.cancel(); deadline = scope.launch { delay(15_000); active.close() } }
            try {
                OperationDeadline.run(if (subscribe) SubscriptionSession.OPERATION_MS else 15_000) { withContext(Dispatchers.IO) {
                    fun current() = AuthenticationProof.current(deviceId, { identity.deviceId }, { ensureActive() })
                    val (hint, raw) = HintDialer.dial(host.hints) { hint, timeout ->
                        val attempt = Socket()
                        try {
                            active.install(attempt)
                            current(); bindSocket(attempt, hint.host); current()
                            attempt.connect(InetSocketAddress(hint.host, hint.port), timeout)
                            attempt
                        } catch (error: Throwable) { active.retire(attempt); throw error }
                    }
                    try {
                        current()
                        val context = SSLContext.getInstance("TLSv1.3")
                        val trust = PinnedTrustManager(host.fingerprint)
                        context.init(null, arrayOf(trust), SecureRandom())
                        // The pin is checked once, on the connected hint; a mismatch never tries another.
                        val socket = context.socketFactory.createSocket(raw, "zenpi.local", hint.port, true) as SSLSocket
                        active.transfer(raw, socket)
                        socket.use {
                            socket.enabledProtocols = arrayOf("TLSv1.3"); socket.soTimeout = 5000
                            socket.sslParameters = socket.sslParameters.apply { applicationProtocols = arrayOf("zenpi/1") }
                            socket.startHandshake()
                            HandshakePrelude.applicationProtocol(socket.applicationProtocol)
                            current()
                            fun envelope(payload: JsonObject) = JsonObject().apply {
                                addProperty("schema_version", 1); addProperty("channel", "control"); add("payload", payload)
                            }
                            FrameCodec.write(socket.outputStream, envelope(JsonObject().apply {
                                addProperty("type", "hello"); add("supported_versions", JsonArray().apply { add(1) })
                            }))
                            HandshakePrelude.welcome(socket.inputStream)
                            current()
                            // Proofs bind the name the host itself uses; it must name the verified key.
                            val (hostName, nonce) = HandshakePrelude.challenge(socket.inputStream,
                                requireNotNull(trust.verified) { "Host certificate unverified" }.names)
                            current()
                            var received: List<JsonObject> = emptyList()
                            var subscriptionPage: (suspend (List<JsonObject>, JsonObject) -> Unit)? = null
                            suspend fun exchange(request: JsonObject): JsonObject {
                                current()
                                val id = if (request.has("id")) FrameCodec.string(request, "id") else UUID.randomUUID().toString()
                                request.addProperty("id", id)
                                FrameCodec.write(socket.outputStream, request)
                                val page = try { ResponseReader.read(socket.inputStream, request, validateResponse = { response ->
                                    current()
                                    if (FrameCodec.string(response, "execution_state") == "failed") {
                                        SessionChangedException.fromSubscription(request, response, workspaceCommand == null && subscribe)?.let { throw it }
                                        val message = "Host request refused: ${FrameCodec.string(response, "code")}"
                                        if (workspaceCommand?.envelope?.get("id")?.asString == id) throw WorkspaceRefusedException(message, FrameCodec.string(response, "code"))
                                        throw HostRefusedException(FrameCodec.string(response, "code"))
                                    }
                                    if (FrameCodec.string(request["payload"].asJsonObject, "type") == "authenticate")
                                        AuthenticationReceipt.validateResponse(response, deviceId)
                                    val operation = workspaceCommand?.envelope?.get("payload")?.asJsonObject?.get("operation")?.asString
                                    val prompt = operation in setOf("prompt", "steer", "cancel", "approve")
                                    val workspaceResponse = workspaceCommand?.envelope?.get("id")?.asString == id
                                    val state = FrameCodec.string(response, "execution_state")
                                    WorkspaceOutcomeCodec.requireKnown(request, response)
                                    if (workspaceResponse && state == "unknown_outcome") throw WorkspaceUnknownOutcomeException()
                                    val accepted = prompt && workspaceResponse && state == "accepted"
                                    if (!accepted && FrameCodec.string(response, "execution_state") != "succeeded") throw WireException("Host request incomplete")
                                    if (workspaceCommand?.envelope?.get("id")?.asString == id) WorkspaceReceiptCodec.validate(workspaceCommand, response)
                                }, onSubscriptionPage = subscriptionPage?.let { consume ->
                                    { batch -> current(); consume(batch.frames, batch.result); current() }
                                }) } catch (error: HostRefusedException) {
                                    val automaticRead = (subscribe || recoverSessionChanges) && workspaceCommand == null && revokeRequestId == null && revokeStatusId == null
                                        && diffPath == null && transcriptRequest == null && filesRequest == null && resourcesRequest == null && modelRequest == null
                                    throw SessionChangeProbe.resolve(request, error, automaticRead, ::exchange)
                                }
                                received = page.frames
                                current()
                                return page.result
                            }
                            if (revokeStatusId != null) {
                                val queryId = UUID.randomUUID().toString()
                                val result = exchange(envelope(JsonObject().apply {
                                    addProperty("type", "revoke_status"); addProperty("device_id", deviceId)
                                    addProperty("request_id", revokeStatusId)
                                    addProperty("signature", AuthenticationProof.sign(deviceId, { identity.deviceId }, { ensureActive() }) {
                                        identity.signRevokeStatus(hostName, nonce, revokeStatusId, queryId)
                                    })
                                }).apply { addProperty("id", queryId) })
                                if (result.keySet() != setOf("device_id", "request_id", "committed") || FrameCodec.string(result, "device_id") != deviceId
                                    || FrameCodec.string(result, "request_id") != revokeStatusId || !result["committed"].isJsonPrimitive
                                    || !result["committed"].asJsonPrimitive.isBoolean) throw WireException("Revocation receipt mismatch")
                                onRevokeStatus(result["committed"].asBoolean)
                                return@use HostProjection(emptyList(), null, null, null, emptyList(), null, false)
                            }
                            val authenticated = exchange(envelope(JsonObject().apply {
                                addProperty("type", "authenticate"); addProperty("device_id", deviceId)
                                addProperty("signature", AuthenticationProof.sign(deviceId, { identity.deviceId }, { ensureActive() }) {
                                    identity.signChallenge(hostName, nonce)
                                })
                            }))
                            val deviceRevision = AuthenticationReceipt.validate(authenticated, deviceId)
                            if (revokeRequestId != null) {
                                val revision = deviceRevision ?: throw WireException("Host self-revoke unavailable")
                                val result = exchange(envelope(JsonObject().apply {
                                    addProperty("type", "request"); addProperty("operation", "device_revoke_self"); add("arguments", JsonObject())
                                }).apply { addProperty("id", revokeRequestId); addProperty("required_owner", deviceId); addProperty("expected_revision", java.math.BigInteger(revision)) })
                                if (result.keySet() != setOf("device_id", "revoked") || FrameCodec.string(result, "device_id") != deviceId
                                    || !result["revoked"].isJsonPrimitive || !result["revoked"].asJsonPrimitive.isBoolean || !result["revoked"].asBoolean)
                                    throw WireException("Host revocation unconfirmed")
                                return@use HostProjection(emptyList(), null, null, null, emptyList(), null, false)
                            }
                            workspaceCommand?.let {
                                val steering = it.envelope["payload"].asJsonObject["operation"].asString == "steer"
                                var receiptPolls = 0
                                while (true) {
                                    try { exchange(it.envelope); break }
                                    catch (error: WireException) {
                                        // Steering admission deliberately starts as unknown until the
                                        // host's sole writer durably acknowledges the queued input.
                                        // The request id is deduplicated on the host, so polling the
                                        // exact envelope cannot submit a second steer. Other workspace
                                        // mutations must surface unknown outcome immediately.
                                        val pendingSteer = error is WorkspaceUnknownOutcomeException && error.code == null
                                            || error.message == "Host request incomplete"
                                        if (!steering || !pendingSteer || receiptPolls++ >= 10) throw error
                                        // Poll the exact durable reservation; never generate a second input identity.
                                        // Keep the bounded receipt poll cancellable; a
                                        // blocking sleep would delay route-loss or
                                        // Activity background cancellation.
                                        delay(150)
                                    }
                                }
                                onWorkspaceConfirmed()
                                current()
                            }
                            fun request(operation: String, project: RemoteProject? = null) = envelope(JsonObject().apply {
                                addProperty("type", "request"); addProperty("operation", operation); add("arguments", JsonObject())
                            }).apply {
                                project?.let { addProperty("project_id", it.id); addProperty("session_id", it.sessionId) }
                            }
                            if (diffPath != null) {
                                val projection = previous ?: throw WireException("Missing diff selection")
                                val command = DiffProjectionCodec.request(projection, diffPath)
                                onDiff(DiffProjectionCodec.decode(exchange(command), projection, diffPath))
                                return@use projection
                            }
                            if (transcriptRequest != null) {
                                onTranscript(exchange(transcriptRequest))
                                return@use previous ?: throw WireException("Missing transcript selection")
                            }
                            if (filesRequest != null) {
                                onFiles(exchange(filesRequest))
                                return@use previous ?: throw WireException("Missing file selection")
                            }
                            if (resourcesRequest != null) {
                                onResources(exchange(resourcesRequest))
                                return@use previous ?: throw WireException("Missing resource selection")
                            }
                            if (modelRequest != null) {
                                onModel(exchange(modelRequest))
                                return@use previous ?: throw WireException("Missing model selection")
                            }
                            val listing = exchange(request("workspace_list"))
                            val capability = listing["event_subscription_available"]
                            if (capability != null && (!capability.isJsonPrimitive || !capability.asJsonPrimitive.isBoolean))
                                throw WireException("Invalid subscription capability")
                            val projects = HostProjectionCodec.projects(listing)
                            val selected = HostProjectionCodec.select(listing, projects, selectedId)
                            val ackFlag = listing["event_ack_available"]
                            if (ackFlag != null && (!ackFlag.isJsonPrimitive || !ackFlag.asJsonPrimitive.isBoolean))
                                throw WireException("Invalid ACK capability")
                            // Only callers with a successfully persisted display/cursor may opt in.
                            if (acknowledgePrevious && workspaceCommand == null && ackFlag?.asBoolean == true
                                && previous?.eventSequence != null && previous.selectedId == selected?.id && previous.sessionId == selected?.sessionId) {
                                val ack = EventAckCodec.request(requireNotNull(previous.selectedId),
                                    requireNotNull(previous.sessionId), previous.eventSequence)
                                EventAckCodec.acknowledge(ack, deviceId, requireNotNull(previous.selectedId),
                                    requireNotNull(previous.sessionId), previous.eventSequence, ::exchange)
                            }
                            val subtabs = selected?.let { SubtabProjectionCodec.decode(exchange(request("subtab_list", it)), it, listing) }
                            if (workspaceCommand?.envelope?.get("payload")?.asJsonObject?.get("operation")?.asString in setOf("prompt", "steer", "cancel", "approve")
                                && previous != null && selected != null && selected.id == previous.selectedId && selected.sessionId == previous.sessionId) {
                                val confirmedView = HostProjectionCodec.workspace(previous.copy(projects = projects, subtabs = subtabs), listing)
                                zenpi.state.ProjectionCommitPolicy.requireCurrent(previous, confirmedView)
                                return@use confirmedView
                            }
                            val initial = run projection@{
                            if (selected == null) HostProjectionCodec.workspace(HostProjection(projects, null, null, null, emptyList(), null, false), listing)
                            else {
                                if (TranscriptRestorePolicy.replay(previous, selected)) {
                                    var current: HostProjection = requireNotNull(previous)
                                    repeat(3) {
                                        val resume = envelope(JsonObject().apply {
                                            addProperty("type", "resume"); addProperty("limit", 128)
                                            add("cursor", JsonObject().apply { addProperty("session_id", selected.sessionId); addProperty("next_sequence", java.math.BigInteger(requireNotNull(current.eventSequence))) })
                                        }).apply { addProperty("project_id", selected.id); addProperty("session_id", selected.sessionId) }
                                        val result = try { exchange(resume) } catch (error: HostRefusedException) {
                                            return@projection TranscriptRestorePolicy.busyView(current, selected, projects, listing, subtabs,
                                                subscribe, capability?.asBoolean == true, workspaceCommand != null, error, busyListing)
                                        }
                                        current = HostProjectionCodec.replay(current, projects, selected, received.toList(), result)
                                        if (!current.hasMoreEvents) return@projection HostProjectionCodec.workspace(current, listing).copy(subtabs = subtabs)
                                    }
                                    HostProjectionCodec.workspace(current, listing).copy(subtabs = subtabs)
                                } else {
                                    val snapshot = try { exchange(request("inspect", selected)) } catch (error: HostRefusedException) {
                                        return@projection TranscriptRestorePolicy.busyView(previous, selected, projects, listing, subtabs,
                                            subscribe, capability?.asBoolean == true, workspaceCommand != null, error, busyListing)
                                    }
                                    HostProjectionCodec.workspace(HostProjectionCodec.snapshot(projects, selected, snapshot), listing).copy(subtabs = subtabs)
                                }
                            }
                            }
                            zenpi.state.ProjectionCommitPolicy.requireCurrent(previous, initial)
                            if (!subscribe || workspaceCommand != null || selected == null || initial.hasMoreEvents
                                || capability?.asBoolean != true || initial.eventSequence == null) return@use initial
                            var streamed = initial
                            onProjection(streamed)
                            current()
                            // ZE-542: consecutive pages reuse this TLS session; each page starts a
                            // fresh reducer from the held projection, as after a reconnect.
                            val session = SubscriptionSession()
                            do {
                                val reducer = SubscriptionReducer(streamed, selected)
                                subscriptionPage = { frames, result ->
                                    val next = reducer.accept(frames, result)
                                    if (next != streamed) onProjection(next)
                                    streamed = next
                                }
                                rearm()
                                exchange(envelope(JsonObject().apply {
                                    addProperty("type", "subscribe"); addProperty("limit", 128); addProperty("duration_ms", SubscriptionSession.PAGE_MS)
                                    add("cursor", JsonObject().apply { addProperty("session_id", selected.sessionId)
                                        addProperty("next_sequence", java.math.BigInteger(requireNotNull(streamed.eventSequence))) })
                                }).apply { addProperty("project_id", selected.id); addProperty("session_id", selected.sessionId) })
                                current()
                            } while (session.continues())
                            streamed
                        }
                    } finally { try { active.close() } finally { raw.close() } }
                } }
            } finally { deadline.cancelAndJoin(); closer.cancelAndJoin() }
        }
    }
}

/**
 * ZE-516 multi-hint dialing: LAN hints first, then the others in QR order. Only a
 * failed bind/connect moves on to the next hint; a pin (host identity) failure is
 * final and never falls back, because a path change must not change who we trust.
 */
internal object HintDialer {
    fun ordered(hints: List<AddressHint>): List<AddressHint> = hints.filter { it.kind == "lan" } + hints.filter { it.kind != "lan" }
    fun <T> dial(hints: List<AddressHint>, attempt: (AddressHint, Int) -> T): Pair<AddressHint, T> {
        val timeout = if (hints.size > 1) 3_000 else 5_000
        var failure: java.io.IOException? = null
        for (hint in ordered(hints)) {
            try { return hint to attempt(hint, timeout) }
            catch (error: java.io.IOException) {
                if (HostIdentityFailure.refused(error)) throw error
                failure?.let(error::addSuppressed)
                failure = error
            }
        }
        throw failure ?: java.io.IOException("No address hint")
    }
}

/**
 * ZE-542: a foreground subscription keeps one authenticated TLS session for consecutive
 * host pages and hands back to the caller (which reconnects after its idle delay) only
 * once [REUSE_MS] has passed, so an idle phone handshakes at most once a minute.
 */
internal class SubscriptionSession(private val clock: () -> Long = System::nanoTime) {
    companion object {
        const val PAGE_MS = 3_000
        const val REUSE_MS = 60_000L
        /** Opening exchange, the reuse window and one last page, each bounded by the 15 s watchdog. */
        const val OPERATION_MS = 15_000L + REUSE_MS + 15_000L
    }
    private val started = clock()
    fun continues(): Boolean = clock() - started < REUSE_MS * 1_000_000
}
