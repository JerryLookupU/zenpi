package zenpi

import com.google.gson.JsonParser
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*

class WorkspaceFilesTest {
    @Test fun retainedMetadataAndPagingCannotAuthorizeReferencesOrNavigationAfterCorruption() {
        val files = WorkspaceFilesCodec.decode(value(), projection, ".")
        val selected = files.entries.last()
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED,
            projection = projection.copy(projects = projection.projects.map { it.copy(promptControl = true) }), files = files)
        for (bad in listOf(selected.copy(name = "other"), selected.copy(sizeBytes = null), selected.copy(sizeBytes = "01"),
            selected.copy(sizeBytes = "18446744073709551616"), selected.copy(sizeBytes = "10485761"),
            selected.copy(directory = true))) {
            assertTrue(runCatching { WorkspaceFilesCodec.reference(bad) }.isFailure)
            val changed = files.copy(entries = listOf(bad))
            assertFalse(WorkspaceFilesCodec.matchesSelection(changed, projection))
            assertFalse(zenpi.pairing.FileReadPolicy.attachment(state.copy(files = changed)))
        }
        val otherDirectory = selected.copy(path = "other/file name.txt")
        assertEquals("@\"other/file name.txt\"", WorkspaceFilesCodec.reference(otherDirectory))
        assertFalse(WorkspaceFilesCodec.matchesSelection(files.copy(entries = listOf(otherDirectory)), projection))
        val first = WorkspaceFilesCodec.decode(page(0, 64, 64), projection, ".")
        val tail = WorkspaceFilesCodec.decode(page(64, 3, null), projection, ".", first, 64)
        val ready = state.copy(files = tail, filesPrevious = listOf(first))
        assertTrue(zenpi.pairing.FileReadPolicy.navigation(ready))
        for (bad in listOf(tail.copy(page = tail.page?.copy(snapshotId = "bad")), tail.copy(truncated = true),
            tail.copy(page = tail.page?.copy(offset = 1)), tail.copy(page = tail.page?.copy(nextOffset = 128)),
            tail.copy(entries = emptyList()))) {
            assertFalse(WorkspaceFilesCodec.matchesSelection(bad, projection))
            assertFalse(zenpi.pairing.FileReadPolicy.navigation(ready.copy(files = bad)))
            assertTrue(runCatching { WorkspaceFilesCodec.request(projection, ".", bad, 0) }.isFailure)
        }
        for (history in listOf(listOf(first.copy(page = first.page?.copy(snapshotId = "e".repeat(64)))),
            listOf(first.copy(entries = first.entries.dropLast(1) + tail.entries.first())), List(8) { first })) {
            assertFalse(zenpi.pairing.FileReadPolicy.navigation(ready.copy(filesPrevious = history)))
        }
        assertTrue(zenpi.pairing.FileReadPolicy.browse(ready.copy(files = tail.copy(truncated = true))))
    }

    @Test fun fullTabListValidityFencesReadDecodeNavigationAttachmentAndLatePublication() {
        val files = WorkspaceFilesCodec.decode(value(), projection, ".")
        val tabs = requireNotNull(projection.subtabs)
        val other = RemoteSubtab("e".repeat(64), "Other", "in_place", 1)
        val control = projection.copy(projects = projection.projects.map { it.copy(promptControl = true) })
        val healthy = control.copy(subtabs = tabs.copy(tabs = tabs.tabs + other))
        val state = zenpi.pairing.PairingState(phase = zenpi.pairing.PairingPhase.CONNECTED, projection = healthy, files = files)
        assertTrue(WorkspaceFilesCodec.matchesSelection(files, healthy))
        assertTrue(zenpi.pairing.FileReadPolicy.attachment(state))
        for (invalid in listOf(tabs.copy(tabs = tabs.tabs + listOf(other, other)),
            tabs.copy(tabs = tabs.tabs + (1..32).map { other.copy(id = "%064x".format(it)) }))) {
            val changed = control.copy(subtabs = invalid)
            assertFalse(WorkspaceFilesCodec.readable(changed))
            assertFalse(WorkspaceFilesCodec.matchesSelection(files, changed))
            assertTrue(runCatching { WorkspaceFilesCodec.request(changed, ".") }.isFailure)
            assertTrue(runCatching { WorkspaceFilesCodec.decode(value(), changed, ".") }.isFailure)
            val blocked = state.copy(projection = changed)
            assertFalse(zenpi.pairing.FileReadPolicy.browse(blocked))
            assertFalse(zenpi.pairing.FileReadPolicy.navigation(blocked))
            assertFalse(zenpi.pairing.FileReadPolicy.attachment(blocked))
            assertEquals(files, state.files)
        }
    }

    private val tab = "b".repeat(64)
    private val projection = HostProjection(listOf(RemoteProject("project", "Project", "session")), "project", "session", "idle",
        emptyList(), "1", false, "0", subtabs = RemoteSubtabs("project", "7", tab, listOf(RemoteSubtab(tab, "main", "main", 1)), false))
    private fun value() = JsonParser.parseString("""{"project_id":"project","session_id":"session","subtab_id":"$tab","subtab_revision":7,"path":".","entries":[{"name":"docs","path":"docs","kind":"directory","size_bytes":null,"attachable":false},{"name":"file name.txt","path":"file name.txt","kind":"file","size_bytes":10485760,"attachable":true}],"truncated":false}""").asJsonObject
    @Test fun readOnlyRequestAndExactSizeBoundaryDecodeWithoutCursorMutation() {
        val request = WorkspaceFilesCodec.request(projection, ".")
        FrameCodec.decode(request.toString().toByteArray())
        assertFalse(request.has("required_owner")); assertFalse(request.has("expected_revision"))
        val files = WorkspaceFilesCodec.decode(value(), projection, ".")
        assertTrue(files.entries[0].directory); assertTrue(files.entries[1].attachable)
        assertEquals("@\"file name.txt\"", WorkspaceFilesCodec.reference(files.entries[1]))
        val over = value(); over["entries"].asJsonArray[1].asJsonObject.apply {
            addProperty("size_bytes", 10485761); addProperty("attachable", false)
        }
        assertFalse(WorkspaceFilesCodec.decode(over, projection, ".").entries[1].attachable)
        assertEquals("0", projection.eventSequence)
    }
    @Test fun malformedScopePathsFlagsSizesAndDuplicatesAreRefused() {
        for (change in listOf<(com.google.gson.JsonObject) -> Unit>(
            { it.addProperty("session_id", "other") }, { it.addProperty("project_id", "other") },
            { it.addProperty("subtab_id", "c".repeat(64)) }, { it.addProperty("subtab_revision", "7") },
            { it.addProperty("path", "other") }, { it.addProperty("truncated", "false") }, { it.addProperty("extra", true) },
            { it["entries"].asJsonArray[1].asJsonObject.addProperty("path", "../outside") },
            { it["entries"].asJsonArray[1].asJsonObject.addProperty("path", "other/file name.txt") },
            { it["entries"].asJsonArray[1].asJsonObject.addProperty("size_bytes", -1) },
            { it["entries"].asJsonArray[1].asJsonObject.addProperty("attachable", false) },
            { it["entries"].asJsonArray[0].asJsonObject.addProperty("attachable", true) },
            { it["entries"].asJsonArray.add(it["entries"].asJsonArray[1].deepCopy()) })) {
            assertTrue(runCatching { WorkspaceFilesCodec.decode(value().apply(change), projection, ".") }.isFailure)
        }
        for (path in listOf("", "/etc", "../outside", "a/../b", "a//b", "a\u0000b", "x".repeat(4097)))
            assertTrue(runCatching { WorkspaceFilesCodec.request(projection, path) }.isFailure)
    }
    @Test fun quotedNativeReferenceAndSelectionFencesRemainExact() {
        val file = WorkspaceFile("a\"b\\c.txt", "docs/a\"b\\c.txt", false, "1", true)
        assertEquals("@\"docs/a\\\"b\\\\c.txt\"", WorkspaceFilesCodec.reference(file))
        assertTrue(WorkspaceFilesCodec.sameSelection(projection, projection.copy(eventSequence = "5")))
        assertFalse(WorkspaceFilesCodec.sameSelection(projection, projection.copy(sessionId = "other")))
        assertFalse(WorkspaceFilesCodec.sameSelection(projection, projection.copy(subtabs = projection.subtabs?.copy(revision = "8"))))
        val many = value(); repeat(63) { many["entries"].asJsonArray.add(many["entries"].asJsonArray[1].deepCopy()) }
        assertTrue(runCatching { WorkspaceFilesCodec.decode(many, projection, ".") }.isFailure)
    }
    private fun page(offset:Int,count:Int,next:Int?)=value().apply {
        add("entries",com.google.gson.JsonArray().apply{repeat(count){index->add(JsonParser.parseString("""{"name":"file-${offset+index}.txt","path":"file-${offset+index}.txt","kind":"file","size_bytes":1,"attachable":true}"""))}})
        add("page",com.google.gson.JsonObject().apply{addProperty("snapshot_id","d".repeat(64));addProperty("offset",offset)
            add("next_offset",next?.let{com.google.gson.JsonPrimitive(it)} ?: com.google.gson.JsonNull.INSTANCE)})
    }
    @Test fun privatePathsCannotBeRequestedReturnedOrInsertedAsReferences() {
        for(path in listOf("credentials.json","auth.json","PRIVATE.KEY","private.pem","cert.p12","id_rsa","docs/auth.json","credentials")) {
            assertTrue(path,runCatching{WorkspaceFilesCodec.request(projection,path)}.isFailure)
            assertTrue(path,runCatching{WorkspaceFilesCodec.reference(WorkspaceFile(path,path,false,"1",true))}.isFailure)
            val bad=value().apply{getAsJsonArray("entries")[1].asJsonObject.apply{addProperty("name",path);addProperty("path",path)}}
            assertTrue(path,runCatching{WorkspaceFilesCodec.decode(bad,projection,".")}.isFailure)
        }
    }
    @Test fun pagesBindSnapshotDirectoryAndScopeAndLegacyRequestsRemainReadOnly() {
        val first=WorkspaceFilesCodec.decode(page(0,64,64),projection,".")
        val request=WorkspaceFilesCodec.request(projection,".",first,64)
        assertEquals("64",request.getAsJsonObject("payload").getAsJsonObject("arguments")["offset"].toString())
        assertFalse(request.has("required_owner"));assertFalse(request.has("expected_revision"))
        val tail=WorkspaceFilesCodec.decode(page(64,3,null),projection,".",first,64)
        assertEquals(64,tail.page?.offset);assertNull(tail.page?.nextOffset)
        assertEquals("file-66.txt",tail.entries.last().path)
        val back=WorkspaceFilesCodec.decode(page(0,64,64),projection,".",tail,0)
        assertEquals(first,back)
        val legacy=WorkspaceFilesCodec.request(projection,".",paged=false)
        assertEquals(setOf("subtab_id","subtab_revision","path"),legacy.getAsJsonObject("payload").getAsJsonObject("arguments").keySet())
        assertNull(WorkspaceFilesCodec.decode(value(),projection,".").page)
        for(change in listOf<(com.google.gson.JsonObject)->Unit>(
            {it.getAsJsonObject("page").addProperty("snapshot_id","e".repeat(64))},
            {it.getAsJsonObject("page").addProperty("offset","64")},
            {it.getAsJsonObject("page").addProperty("next_offset",128)},
            {it.getAsJsonObject("page").addProperty("extra",true)},
            {it.addProperty("truncated",true)}, {it.remove("page")})) {
            assertTrue(runCatching{WorkspaceFilesCodec.decode(page(64,3,null).apply(change),projection,".",first,64)}.isFailure)
        }
        assertTrue(runCatching{WorkspaceFilesCodec.request(projection,"docs",first,64)}.isFailure)
        assertTrue(runCatching{WorkspaceFilesCodec.request(projection.copy(sessionId="foreign"),".",first,64)}.isFailure)
        assertTrue(runCatching{WorkspaceFilesCodec.request(projection,".",offset=64)}.isFailure)
        assertTrue(runCatching{WorkspaceFilesCodec.decode(page(64,0,null),projection,".",first,64)}.isFailure)
        assertTrue(runCatching{WorkspaceFilesCodec.decode(page(0,63,64),projection,".")}.isFailure)
    }
    @Test fun cachedPreviousPageRequiresSameSnapshotAndPathScope() {
        val first = WorkspaceFilesCodec.decode(page(0, 64, 64), projection, ".")
        val second = WorkspaceFilesCodec.decode(page(64, 3, null), projection, ".", first, 64)
        assertTrue(WorkspaceFilesCodec.samePageScope(first, second))
        assertFalse(WorkspaceFilesCodec.samePageScope(first, second.copy(path = "docs")))
        assertFalse(WorkspaceFilesCodec.samePageScope(first, second.copy(sessionId = "other")))
        assertFalse(WorkspaceFilesCodec.samePageScope(first, second.copy(page = second.page?.copy(snapshotId = "e".repeat(64)))))
    }
    @Test fun skippedTerminalUnanchoredAndRepeatedFilePagesNeverBecomeSelections() {
        val first = WorkspaceFilesCodec.decode(page(0, 64, 64), projection, ".")
        val last = WorkspaceFilesCodec.decode(page(64, 3, null), projection, ".", first, 64)
        for ((previous, offset) in listOf(null to 64, first to 128, first to 0, last to 128, last to 64, last to 2048)) {
            assertTrue(runCatching { WorkspaceFilesCodec.request(projection, ".", previous, offset) }.isFailure)
            assertTrue(runCatching { WorkspaceFilesCodec.decode(page(offset, 1, null), projection, ".", previous, offset) }.isFailure)
        }
        val repeated = page(64, 1, null).apply { getAsJsonArray("entries")[0].asJsonObject.apply {
            addProperty("name", "file-0.txt"); addProperty("path", "file-0.txt")
        } }
        assertTrue(runCatching { WorkspaceFilesCodec.decode(repeated, projection, ".", first, 64) }.isFailure)
        assertTrue(WorkspaceFilesCodec.matchesSelection(last, projection))
        for (changed in listOf(last.copy(projectId = "foreign"), last.copy(sessionId = "foreign"),
            last.copy(subtabId = "c".repeat(64)), last.copy(subtabRevision = "8"))) {
            assertFalse(WorkspaceFilesCodec.matchesSelection(changed, projection))
        }
        assertFalse(WorkspaceFilesCodec.matchesSelection(last, null))
    }
    @Test fun retainedFilePagesStayConsecutiveDistinctAndBounded() {
        fun previous(offset: Int) = WorkspaceFiles("project", "session", tab, "7", ".", (offset until offset + 64).map {
            WorkspaceFile("file-$it.txt", "file-$it.txt", false, "1", true) }, false, WorkspaceFilesPage("d".repeat(64), offset, offset + 64))
        fun files(offset: Int) = WorkspaceFilesCodec.decode(page(offset, 64, offset + 64), projection, ".",
            if (offset == 0) null else previous(offset - 64), offset)
        var history = emptyList<WorkspaceFiles>()
        var current = files(0)
        for (offset in 64..576 step 64) {
            val next = files(offset)
            history = WorkspaceFilesCodec.remember(history, current, next)
            current = next
        }
        assertEquals((128..512 step 64).toList(), history.map { it.page!!.offset })
        assertTrue(runCatching { WorkspaceFilesCodec.remember(history, current, files(640).copy(entries = history.first().entries)) }.isFailure)
        assertTrue(runCatching { WorkspaceFilesCodec.remember(history.reversed(), current, files(640)) }.isFailure)
        assertTrue(runCatching { WorkspaceFilesCodec.remember(history, current, files(704)) }.isFailure)
        assertTrue(runCatching { WorkspaceFilesCodec.remember(history, current, files(640).copy(path = "docs")) }.isFailure)
        assertEquals(emptyList<WorkspaceFiles>(), WorkspaceFilesCodec.remember(history, null, files(0)))
    }
}
