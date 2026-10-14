package zenpi.transport

import com.google.gson.JsonObject
import java.util.UUID

data class WorkspaceFile(val name: String, val path: String, val directory: Boolean, val sizeBytes: String?, val attachable: Boolean)
data class WorkspaceFiles(val projectId: String, val sessionId: String, val subtabId: String, val subtabRevision: String,
    val path: String, val entries: List<WorkspaceFile>, val truncated: Boolean, val page: WorkspaceFilesPage? = null)
data class WorkspaceFilesPage(val snapshotId: String, val offset: Int, val nextOffset: Int?)

internal object WorkspaceFilesCodec {
    fun readable(projection: HostProjection): Boolean = SubtabProjectionCodec.current(projection) != null
    fun privatePath(value:String)=value.split('/').any { raw ->
        val name=raw.lowercase(java.util.Locale.ROOT)
        name in setOf(".git",".zenpi",".ssh",".aws",".azure",".config",".codex",".claude",".netrc",".npmrc",
            "credentials","credentials.json","auth.json","id_rsa","id_ed25519") || name==".env" || name.startsWith(".env.")
            || name.endsWith(".pem") || name.endsWith(".key") || name.endsWith(".p12")
    }
    private fun path(value: String): Boolean = value.isNotEmpty() && value.toByteArray().size <= 4096
        && (value == "." || !value.startsWith('/') && value.split('/').none { it.startsWith('.') || it.isEmpty() })
        && value.none { Character.isISOControl(it) }
        && !privatePath(value)
    private fun validFile(file: WorkspaceFile): Boolean {
        if (file.name.isEmpty() || file.name.startsWith('.') || file.name.contains('/') || !path(file.path)
            || file.path.substringAfterLast('/') != file.name) return false
        if (file.directory) return file.sizeBytes == null && !file.attachable
        val size = file.sizeBytes ?: return false
        return size.matches(Regex("0|[1-9][0-9]*")) && size.toBigInteger() <= java.math.BigInteger("18446744073709551615")
            && file.attachable == (size.toBigInteger() <= java.math.BigInteger.valueOf(10L * 1024 * 1024))
    }
    private fun validFiles(files: WorkspaceFiles): Boolean {
        if (!path(files.path) || files.entries.size > 64 || !files.entries.all { validFile(it)
                && it.path == (if (files.path == ".") it.name else "${files.path}/${it.name}") }
            || files.entries.map { it.path }.distinct().size != files.entries.size) return false
        val page = files.page ?: return true
        return !files.truncated && page.snapshotId.matches(Regex("[0-9a-f]{64}"))
            && page.offset in 0..1984 && page.offset % 64 == 0 && (page.offset == 0 || files.entries.isNotEmpty())
            && (page.nextOffset == null || page.nextOffset == page.offset + 64 && page.nextOffset < 2048 && files.entries.size == 64)
    }
    private fun consecutive(first: WorkspaceFiles, second: WorkspaceFiles): Boolean = samePageScope(first, second)
        && first.page?.nextOffset != null && first.page.nextOffset == second.page?.offset
    fun validChain(pages: List<WorkspaceFiles>, current: WorkspaceFiles): Boolean {
        if (pages.size > 7 || !validFiles(current) || !pages.all(::validFiles)
            || !pages.zipWithNext().all { (first, second) -> consecutive(first, second) }
            || pages.lastOrNull()?.let { consecutive(it, current) } == false) return false
        val paths = (pages + current).flatMap { it.entries }.map { it.path }
        return paths.distinct().size == paths.size
    }
    fun request(projection: HostProjection, path: String, previous: WorkspaceFiles? = null, offset: Int = 0, paged: Boolean = true): JsonObject {
        require(path(path))
        require(offset in 0..1984 && offset % 64 == 0)
        val tabs = requireNotNull(projection.subtabs)
        require(readable(projection))
        if(previous!=null)require(matchesSelection(previous, projection) && previous.path==path && previous.page!=null
            && (offset == previous.page.nextOffset || previous.page.offset > 0 && offset == previous.page.offset - 64))
        else require(offset==0)
        require(paged || previous==null && offset==0)
        return JsonObject().apply {
            addProperty("schema_version", 1); addProperty("channel", "control"); addProperty("id", UUID.randomUUID().toString())
            addProperty("project_id", projection.selectedId); addProperty("session_id", projection.sessionId)
            add("payload", JsonObject().apply { addProperty("type", "request"); addProperty("operation", "files_list")
                add("arguments", JsonObject().apply { addProperty("subtab_id", tabs.activeId)
                    addProperty("subtab_revision", java.math.BigInteger(tabs.revision)); addProperty("path", path)
                    if(paged){addProperty("offset",offset);add("snapshot_id",previous?.page?.snapshotId?.let{com.google.gson.JsonPrimitive(it)} ?: com.google.gson.JsonNull.INSTANCE)} }) })
        }
    }
    fun sameSelection(projection: HostProjection, next: HostProjection?): Boolean = next != null && projection.selectedId == next.selectedId
        && projection.sessionId == next.sessionId && projection.subtabs?.activeId == next.subtabs?.activeId
        && projection.subtabs?.revision == next.subtabs?.revision
    fun matchesSelection(files: WorkspaceFiles, projection: HostProjection?): Boolean = validFiles(files) && projection != null && readable(projection)
        && files.projectId == projection.selectedId && files.sessionId == projection.sessionId
        && projection.projects.singleOrNull { it.id == files.projectId && it.sessionId == files.sessionId } != null
        && projection.subtabs?.projectId == files.projectId && files.subtabId == projection.subtabs.activeId
        && files.subtabRevision == projection.subtabs.revision
    fun samePageScope(first: WorkspaceFiles, second: WorkspaceFiles): Boolean = first.projectId == second.projectId
        && first.sessionId == second.sessionId && first.subtabId == second.subtabId
        && first.subtabRevision == second.subtabRevision && first.path == second.path
        && first.page?.snapshotId == second.page?.snapshotId
    fun remember(pages: List<WorkspaceFiles>, current: WorkspaceFiles?, next: WorkspaceFiles): List<WorkspaceFiles> {
        if (current == null) { require(validFiles(next) && (next.page == null || next.page.offset == 0)); return emptyList() }
        require(validChain(pages, current) && validFiles(next) && consecutive(current, next))
        val retained = pages + current
        require(retained.flatMap { it.entries }.none { old -> next.entries.any { it.path == old.path } })
        return retained.takeLast(7)
    }
    fun decode(value: JsonObject, projection: HostProjection, requested: String, previous: WorkspaceFiles? = null, offset: Int = 0): WorkspaceFiles {
        request(projection, requested, previous, offset)
        val tabs = requireNotNull(projection.subtabs)
        val fields=setOf("project_id", "session_id", "subtab_id", "subtab_revision", "path", "entries", "truncated")
        if ((value.keySet() != fields && value.keySet()!=fields+"page")
            || FrameCodec.string(value, "project_id") != projection.selectedId || FrameCodec.string(value, "session_id") != projection.sessionId
            || FrameCodec.string(value, "subtab_id") != tabs.activeId || unsigned(value, "subtab_revision") != tabs.revision
            || FrameCodec.string(value, "path") != requested) throw WireException("File listing scope mismatch")
        val raw = value["entries"].takeIf { it.isJsonArray }?.asJsonArray ?: throw WireException("Invalid file listing")
        if (raw.size() > 64) throw WireException("File listing limit")
        val entries = raw.map { element ->
            val item = element.takeIf { it.isJsonObject }?.asJsonObject ?: throw WireException("Invalid file")
            if (item.keySet() != setOf("name", "path", "kind", "size_bytes", "attachable")) throw WireException("Invalid file fields")
            val name = FrameCodec.string(item, "name"); val relative = FrameCodec.string(item, "path"); val kind = FrameCodec.string(item, "kind")
            if (name.isEmpty() || name.startsWith('.') || name.contains('/') || !path(relative)
                || relative != (if (requested == ".") name else "$requested/$name") || kind !in setOf("file", "directory")) throw WireException("Invalid file path")
            val size = if (item["size_bytes"].isJsonNull) null else unsigned(item, "size_bytes")
            val attachable = flag(item, "attachable")
            if (kind == "directory" && (size != null || attachable) || kind == "file" && (size == null
                    || attachable != (java.math.BigInteger(size) <= java.math.BigInteger.valueOf(10L * 1024 * 1024)))) throw WireException("Invalid file size")
            WorkspaceFile(name, relative, kind == "directory", size, attachable)
        }
        if (entries.map { it.path }.distinct().size != entries.size) throw WireException("Duplicate file")
        if (previous?.entries?.any { old -> entries.any { it.path == old.path } } == true) throw WireException("Repeated file page entry")
        val page=if(value.has("page")) {
            val page=value["page"].takeIf{it.isJsonObject}?.asJsonObject ?: throw WireException("Invalid file page")
            if(page.keySet()!=setOf("snapshot_id","offset","next_offset") || unsigned(page,"offset")!=offset.toString()
                || offset !in 0..2048 || offset%64!=0 || offset>0 && entries.isEmpty() || flag(value,"truncated"))throw WireException("Invalid file page")
            val snapshot=FrameCodec.string(page,"snapshot_id")
            if(!snapshot.matches(Regex("[0-9a-f]{64}")) || previous!=null && (previous.page?.snapshotId!=snapshot || previous.path!=requested
                || previous.projectId!=projection.selectedId || previous.sessionId!=projection.sessionId || previous.subtabId!=tabs.activeId || previous.subtabRevision!=tabs.revision))throw WireException("File page scope mismatch")
            val next=if(page["next_offset"].isJsonNull)null else unsigned(page,"next_offset").toIntOrNull() ?: throw WireException("Invalid file page offset")
            if(next!=null && (next!=offset+64 || next>=2048 || entries.size!=64))throw WireException("Invalid file page continuation")
            WorkspaceFilesPage(snapshot,offset,next)
        }else{if(previous!=null || offset!=0)throw WireException("Missing file page");null}
        return WorkspaceFiles(requireNotNull(projection.selectedId), requireNotNull(projection.sessionId), tabs.activeId, tabs.revision, requested, entries, flag(value, "truncated"),page)
            .also { require(validFiles(it)) }
    }
    fun reference(file: WorkspaceFile): String {
        require(validFile(file) && file.attachable && !file.directory)
        return "@\"${file.path.replace("\\", "\\\\").replace("\"", "\\\"")}\""
    }
    private fun flag(value: JsonObject, key: String): Boolean {
        val raw = value[key]
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isBoolean) throw WireException("Invalid file flag")
        return raw.asBoolean
    }
    private fun unsigned(value: JsonObject, key: String): String {
        val raw = value[key]
        if (!raw.isJsonPrimitive || !raw.asJsonPrimitive.isNumber || !raw.toString().matches(Regex("0|[1-9][0-9]*"))
            || raw.asBigInteger > java.math.BigInteger("18446744073709551615")) throw WireException("Invalid file number")
        return raw.toString()
    }
}
