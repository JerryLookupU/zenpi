package zenpi.transport

import com.google.gson.JsonObject
import java.util.UUID

data class WorkerStatusPage(val projectId: String, val sessionId: String, val digest: String, val offset: Int,
    val nextOffset: Int, val totalTasks: Int, val hasMore: Boolean, val tasks: List<RemoteWorkerStatus>)

internal object WorkerStatusPageCodec {
    private fun validPage(page: WorkerStatusPage): Boolean = page.digest.matches(Regex("[0-9a-f]{64}"))
        && page.offset in 0..4096 && page.nextOffset in page.offset..4096 && page.totalTasks in page.nextOffset..4096
        && page.tasks.size <= 16 && page.nextOffset - page.offset == page.tasks.size
        && page.hasMore == (page.nextOffset < page.totalTasks) && (!page.hasMore || page.tasks.isNotEmpty())
        && (page.tasks.isNotEmpty() || page.offset == 0 && page.totalTasks == 0)
        && page.tasks.map { it.requestId }.distinct().size == page.tasks.size
        && page.tasks.all { WorkerStatusCodec.validMetadata(it, page.projectId, page.sessionId) }
    private fun consecutive(first: WorkerStatusPage, second: WorkerStatusPage): Boolean =
        first.projectId == second.projectId && first.sessionId == second.sessionId
            && first.digest == second.digest && first.totalTasks == second.totalTasks
            && first.hasMore && first.nextOffset == second.offset
    fun validChain(pages: List<WorkerStatusPage>, current: WorkerStatusPage): Boolean {
        if (pages.size > 7 || !validPage(current) || !pages.all(::validPage)
            || !pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val ids = (pages + current).flatMap { it.tasks }.map { it.requestId }
        return ids.distinct().size == ids.size
    }
    fun readable(projection: HostProjection): Boolean = projection.selectedId != null && projection.sessionId != null
        && projection.projects.singleOrNull { it.id == projection.selectedId }?.sessionId == projection.sessionId
    fun remember(pages: List<WorkerStatusPage>, current: WorkerStatusPage?, next: WorkerStatusPage): List<WorkerStatusPage> {
        if (current == null) { require(validPage(next) && next.offset == 0); return emptyList() }
        require(validChain(pages, current) && validPage(next) && consecutive(current, next))
        val retained = pages + current
        require(retained.flatMap { it.tasks }.none { old -> next.tasks.any { it.requestId == old.requestId } })
        return retained.takeLast(7)
    }
    fun sameScope(page: WorkerStatusPage, projection: HostProjection?): Boolean = validPage(page) && projection != null && readable(projection)
        && page.projectId == projection.selectedId && page.sessionId == projection.sessionId
    fun previousPage(current: WorkerStatusPage, history: List<WorkerStatusPage>, projection: HostProjection?, offset: Int): WorkerStatusPage? {
        if (!validChain(history, current)) return null
        val previous = history.lastOrNull() ?: return null
        return previous.takeIf { offset == it.offset && it.nextOffset == current.offset
            && sameScope(it, projection) && sameScope(current, projection)
            && it.digest == current.digest && it.totalTasks == current.totalTasks }
    }
    fun request(projection: HostProjection, previous: WorkerStatusPage?, offset: Int): JsonObject {
        require(readable(projection) && offset in 0..4096)
        require(previous == null && offset == 0 || previous != null && previous.hasMore
            && sameScope(previous, projection) && offset == previous.nextOffset)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "workspace_list")
                add("arguments", JsonObject().apply { addProperty("view", "worker_status"); addProperty("offset", offset)
                    add("digest", previous?.digest?.let { com.google.gson.JsonPrimitive(it) } ?: com.google.gson.JsonNull.INSTANCE) }) })
        }
    }
    fun decode(value: JsonObject, projection: HostProjection, previous: WorkerStatusPage?, offset: Int): WorkerStatusPage {
        if (!readable(projection)) throw WireException("Task list selection unavailable")
        if (previous == null && offset != 0 || previous != null && (!previous.hasMore || !sameScope(previous, projection)
                || offset != previous.nextOffset)) throw WireException("Task list continuity mismatch")
        if (value.keySet() != setOf("project_id", "session_id", "digest", "offset", "next_offset", "total_tasks", "has_more", "tasks")
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId) throw WireException("Task list scope mismatch")
        fun number(field: String): Int = value[field].let {
            if (!it.isJsonPrimitive || !it.asJsonPrimitive.isNumber || !it.toString().matches(Regex("0|[1-9][0-9]*"))) throw WireException("Invalid task offset")
            it.toString().toIntOrNull()?.takeIf { n -> n in 0..4096 } ?: throw WireException("Task list limit")
        }
        val start = number("offset"); val next = number("next_offset"); val total = number("total_tasks")
        val digest = FrameCodec.string(value, "digest"); val more = value["has_more"]
        if (!value.has("tasks")) throw WireException("Missing task list")
        val tasks = WorkerStatusCodec.decode(value["tasks"], requireNotNull(projection.selectedId), requireNotNull(projection.sessionId))
        if (!digest.matches(Regex("[0-9a-f]{64}")) || !more.isJsonPrimitive || !more.asJsonPrimitive.isBoolean || start != offset
            || next - start != tasks.size || next > total || more.asBoolean != (next < total) || more.asBoolean && tasks.isEmpty()
            || tasks.any { it.outputText != null || it.outputWithheld || it.streamText != null }
            || previous != null && (previous.digest != digest || previous.totalTasks != total
                || previous.tasks.any { old -> tasks.any { it.requestId == old.requestId } })) throw WireException("Task list changed")
        return WorkerStatusPage(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), digest, start, next, total, more.asBoolean, tasks)
            .also { if (!validPage(it)) throw WireException("Invalid task metadata page") }
    }
}
