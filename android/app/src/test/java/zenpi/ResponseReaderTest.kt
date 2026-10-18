package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import java.io.ByteArrayInputStream
import java.io.ByteArrayOutputStream
import java.io.InputStream
import java.net.SocketTimeoutException
import zenpi.transport.*

class ResponseReaderTest {
    private val project = RemoteProject("project", "Workspace", "session")
    private val initial = HostProjection(listOf(project), "project", "session", "idle", emptyList(), "0", false, "0")
    private fun request(type: String = "subscribe") = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","project_id":"project","session_id":"session","payload":{"type":"$type","cursor":{"session_id":"session","next_sequence":0},"limit":128${if (type == "subscribe") ",\"duration_ms\":1000" else ""}}}""").asJsonObject
    private fun response(next: Int = 0, done: Boolean = false) = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","execution_state":"succeeded","payload":{"type":"response","result":{"cursor":{"session_id":"session","next_sequence":$next},"cursor_scope":"transcript_events_v1","snapshot_required":false,"has_more":false,"phase":"idle","journal_next_sequence":$next,"subscription_done":$done}}}""").asJsonObject
    private fun event(sequence: Int = 0, content: String = "answer") = JsonParser.parseString("""{"schema_version":1,"channel":"event","session_id":"session","project_id":"project","turn_id":"turn-$sequence","sequence":$sequence,"payload":{"type":"event","name":"transcript_turn","data":{"id":"turn-$sequence","role":"assistant","content":"answer"}}}""").asJsonObject.apply {
        getAsJsonObject("payload").getAsJsonObject("data").addProperty("content", content)
    }
    private fun wire(vararg frames: JsonObject) = ByteArrayOutputStream().also { out -> frames.forEach { FrameCodec.write(out, it) } }.toByteArray()
    private class Fragmented(bytes: ByteArray) : ByteArrayInputStream(bytes) {
        override fun read(target: ByteArray, offset: Int, length: Int): Int = super.read(target, offset, minOf(length, 3))
    }
    private fun validate(response: JsonObject) {
        if (FrameCodec.string(response, "execution_state") != "succeeded") throw WireException("Host refused page")
    }
    @Test fun keyLossWhileReadingAReceiptCannotConfirmOrPublishIt() = runBlocking {
        val device = "a".repeat(64)
        for (deleted in listOf(true, false)) for (type in listOf("resume", "subscribe")) {
            var lost = false; var confirmed = false; var published = false
            val bytes = wire(response(done = true))
            val input = object : ByteArrayInputStream(bytes) {
                override fun read(target: ByteArray, offset: Int, length: Int): Int {
                    lost = true
                    return super.read(target, offset, minOf(length, 3))
                }
            }
            val error = runCatching {
                ResponseReader.read(input, request(type), {
                    AuthenticationProof.current(device, {
                        if (!lost) device else if (deleted) throw java.security.KeyStoreException("PRIVATE_ALIAS") else "b".repeat(64)
                    }, {})
                    validate(it); confirmed = true
                }) { published = true }
            }.exceptionOrNull()
            assertTrue(error is HostAuthenticationException)
            assertFalse(confirmed); assertFalse(published)
            assertFalse(error.toString().contains("PRIVATE"))
        }
    }
    @Test fun keyLossAfterAPublishedPageRetainsThatCursorAndStopsLaterPages() = runBlocking {
        val device = "a".repeat(64)
        val reducer = SubscriptionReducer(initial, project)
        var current = true; var published = 0
        val input = Fragmented(wire(event(), response(1), event(1), response(2, true)))
        val error = runCatching {
            ResponseReader.read(input, request(), {
                AuthenticationProof.current(device, { if (current) device else "b".repeat(64) }, {})
                validate(it)
            }) { page -> reducer.accept(page.frames, page.result); published++; current = false }
        }.exceptionOrNull()
        assertTrue(error is HostAuthenticationException); assertEquals(1, published)
        assertEquals("1", reducer.projection.eventSequence); assertEquals(1, reducer.projection.turns.size)
        assertFalse(reducer.done)
    }
    @Test fun fragmentedMultiPageWireCommitsOnlyCompleteCorrelatedPages() = runBlocking {
        val reducer = SubscriptionReducer(initial, project)
        val published = mutableListOf<HostProjection>()
        val input = Fragmented(wire(response(), event(), response(1), response(1, true)))
        val final = ResponseReader.read(input, request(), ::validate) { page ->
            published.add(reducer.accept(page.frames, page.result))
        }
        assertEquals(listOf("0", "1", "1"), published.map { it.eventSequence })
        assertEquals("answer", reducer.projection.turns.single().content)
        assertTrue(reducer.done); assertTrue(final.frames.isEmpty()); assertEquals(-1, input.read())
    }
    @Test fun everyPartialPageRetainsPriorCommittedDisplay() = runBlocking {
        val first = wire(response())
        val second = wire(event(), response(1, true))
        for (cut in 0 until second.size) {
            val reducer = SubscriptionReducer(initial, project)
            var published = 0
            val error = runCatching {
                ResponseReader.read(Fragmented(first + second.copyOf(cut)), request(), ::validate) { page ->
                    reducer.accept(page.frames, page.result); published++
                }
            }.exceptionOrNull()
            assertNotNull("cut $cut", error)
            assertEquals(1, published); assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
    }
    @Test fun timeoutAfterEventCannotPublishOrAdvanceCursor() = runBlocking {
        val bytes = wire(response(), event())
        val input = object : InputStream() {
            var position = 0
            override fun read(): Int = if (position < bytes.size) bytes[position++].toInt() and 255
                else throw SocketTimeoutException("fixture deadline")
        }
        val reducer = SubscriptionReducer(initial, project)
        var published = 0
        val error = runCatching {
            ResponseReader.read(input, request(), ::validate) { page -> reducer.accept(page.frames, page.result); published++ }
        }.exceptionOrNull()
        assertTrue(error is SocketTimeoutException); assertEquals(1, published)
        assertEquals(initial, reducer.projection)
    }
    @Test fun wrongCorrelationAndRefusedOutcomeNeverReachPublication() = runBlocking {
        for (bad in listOf(response(1, true).apply { addProperty("id", "foreign") },
            response(1, true).apply { addProperty("execution_state", "failed"); addProperty("code", "unauthorized") })) {
            var published = 0
            assertTrue(runCatching {
                ResponseReader.read(Fragmented(wire(event(), bad)), request(), ::validate) { published++ }
            }.isFailure)
            assertEquals(0, published)
        }
    }
    @Test fun responseRoutingFieldsAreRejectedBeforePublication() = runBlocking {
        val bad = response().apply { addProperty("session_id", "foreign") }
        var published = 0
        assertTrue(runCatching { ResponseReader.read(Fragmented(wire(bad)), request(), ::validate) { published++ } }.isFailure)
        assertEquals(0, published)
    }
    @Test fun responseRoutingFieldsMatchingRequestRemainCompatible() = runBlocking {
        val routed = response().apply {
            addProperty("project_id", "project")
            addProperty("session_id", "session")
        }
        val page = ResponseReader.read(Fragmented(wire(routed)), request("resume"), ::validate)
        assertEquals("session", page.result["cursor"].asJsonObject["session_id"].asString)
    }
    @Test fun successWithAnErrorCodeCannotValidateOrPublishAReplayPage() = runBlocking {
        for (code in listOf("busy", "unauthorized", "revoked_device", "session_changed", "internal_error")) {
            for (type in listOf("resume", "subscribe")) {
                val contradictory = response(1, true).apply { addProperty("code", code) }
                var validated = false; var published = false
                val error = runCatching {
                    ResponseReader.read(Fragmented(wire(event(), contradictory)), request(type), { validated = true }) { published = true }
                }.exceptionOrNull()
                assertTrue(error is WireException); assertFalse(validated); assertFalse(published)
            }
        }
        val compatible = response(done = true).apply { add("code", com.google.gson.JsonNull.INSTANCE) }
        assertTrue(ResponseReader.read(Fragmented(wire(compatible)), request("resume"), ::validate).frames.isEmpty())
    }
    @Test fun contradictoryLaterPageKeepsTheLastCommittedDisplayAndCursor() = runBlocking {
        val reducer = SubscriptionReducer(initial, project)
        var published = 0
        val contradictory = response(2, true).apply { addProperty("code", "revoked_device") }
        val error = runCatching {
            ResponseReader.read(Fragmented(wire(event(), response(1), event(1), contradictory)), request(), ::validate) { page ->
                reducer.accept(page.frames, page.result); published++
            }
        }.exceptionOrNull()
        assertTrue(error is WireException); assertEquals(1, published)
        assertEquals("1", reducer.projection.eventSequence); assertEquals(1, reducer.projection.turns.size)
        assertFalse(reducer.done)
    }
    @Test fun failedResponseWithNullResultNeverCrashesOrPublishes() = runBlocking {
        val failed = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","execution_state":"failed","code":"busy","payload":{"type":"response","result":null}}""").asJsonObject
        var published = 0
        assertTrue(runCatching { ResponseReader.read(ByteArrayInputStream(wire(failed)), request(), {}) { published++ } }.isFailure)
        assertEquals(0, published)
    }
    @Test fun eventScopeGapAndAggregateByteOverflowCannotReplaceProjection() = runBlocking {
        for (frames in listOf(arrayOf(event(1)), arrayOf(event().apply { addProperty("session_id", "foreign") }),
            arrayOf(event(0, "x".repeat(100_000)), event(1, "y".repeat(100_000))))) {
            val reducer = SubscriptionReducer(initial, project)
            assertTrue(runCatching {
                ResponseReader.read(Fragmented(wire(*frames, response(frames.size, true))), request(), ::validate) { page ->
                    reducer.accept(page.frames, page.result)
                }
            }.isFailure)
            assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        }
    }
    @Test fun lastBatchMustCompleteAndOrdinaryReplayReturnsOnePage() = runBlocking {
        for (done in listOf(false, true)) {
            var published = 0
            val frames = (0 until 32).map { response(done = done && it == 31) }.toTypedArray()
            val outcome = runCatching {
                ResponseReader.read(Fragmented(wire(*frames)), request(), ::validate) { published++ }
            }
            assertEquals(done, outcome.isSuccess); assertEquals(if (done) 32 else 31, published)
        }
        val input = Fragmented(wire(event(), response(1, true), response()))
        val replay = ResponseReader.read(input, request("resume"), ::validate) { fail("Replay is not subscription") }
        assertEquals(1, replay.frames.size); assertNotNull(FrameCodec.read(input))
    }
    @Test fun publicationFailureStopsBeforeReadingOrPublishingNextPage() = runBlocking {
        val first = wire(response())
        for (failure in listOf(java.io.IOException("storage refused"), kotlinx.coroutines.CancellationException("replaced"))) {
            val input = Fragmented(first + wire(event(), response(1, true)))
            var calls = 0
            val error = runCatching {
                ResponseReader.read(input, request(), ::validate) { calls++; throw failure }
            }.exceptionOrNull()
            assertSame(failure, error); assertEquals(1, calls)
            assertEquals(event(), FrameCodec.read(input))
        }
    }
    @Test fun rawWhitespaceBytesAcrossPagesCannotHideBehindNormalizedJsonBudget() = runBlocking {
        fun padded(frame: JsonObject, spaces: Int): ByteArray {
            val bytes = (frame.toString().dropLast(1) + " ".repeat(spaces) + "}").toByteArray(Charsets.UTF_8)
            return ByteArrayOutputStream().also { java.io.DataOutputStream(it).apply { writeInt(bytes.size); write(bytes) } }.toByteArray()
        }
        val first = padded(response(), 100_000)
        val second = padded(response(done = true), 100_000)
        val input = ByteArrayInputStream(first + second)
        var publications = 0
        val reducer = SubscriptionReducer(initial, project)
        val failure = runCatching {
            ResponseReader.read(input, request(), ::validate) { page ->
                reducer.accept(page.frames, page.result); publications++
            }
        }.exceptionOrNull()
        assertTrue(failure is WireException); assertEquals("Subscription wire limit", failure?.message)
        assertEquals(1, publications); assertEquals(initial, reducer.projection); assertFalse(reducer.done)
        // Refusal happens at the prefix, before reading/allocating the second body.
        assertEquals(second.size - 4, input.available())
        val ordinary = ResponseReader.read(ByteArrayInputStream(padded(response(done = true), 200_000)), request("resume"), ::validate)
        assertTrue(ordinary.frames.isEmpty())
    }
    @Test fun subscriptionActualByteBoundaryIncludesTerminalControlFrames() = runBlocking {
        val frame = response(done = true).toString()
        for (excess in listOf(0, 1)) {
            val bytes = (frame.dropLast(1) + " ".repeat(196_608 + excess - frame.toByteArray().size) + "}").toByteArray()
            val wire = ByteArrayOutputStream().also { java.io.DataOutputStream(it).apply { writeInt(bytes.size); write(bytes) } }.toByteArray()
            var publications = 0
            val input = ByteArrayInputStream(wire)
            val outcome = runCatching { ResponseReader.read(input, request(), ::validate) { publications++ } }
            assertEquals(excess == 0, outcome.isSuccess)
            assertEquals(if (excess == 0) 1 else 0, publications)
            assertEquals(if (excess == 0) 0 else bytes.size, input.available())
        }
    }
    @Test fun ordinaryReplayEventBudgetUsesReceivedBytesRatherThanNormalizedContent() = runBlocking {
        val frames = listOf(event(0), event(1))
        val bytes = ByteArrayOutputStream().also { output ->
            frames.forEach { frame ->
                val body = (frame.toString().dropLast(1) + " ".repeat(100_000) + "}").toByteArray()
                java.io.DataOutputStream(output).apply { writeInt(body.size); write(body) }
            }
            FrameCodec.write(output, response(2, true))
        }.toByteArray()
        var validated = 0
        val failure = runCatching {
            ResponseReader.read(ByteArrayInputStream(bytes), request("resume"), { validated++ })
        }.exceptionOrNull()
        assertEquals("Replay wire limit", failure?.message); assertEquals(0, validated)
    }
    @Test fun fragmentedWorkspaceRollbackLeavesTheLastCompleteAuthorityAndCursorUnchanged() = runBlocking {
        fun view(revision: Int) = JsonParser.parseString("""{"listing":{"mutations_available":false,"active_project_id":"project","required_owner":"${"b".repeat(64)}","workspace_revision":$revision,"projects":[{"id":"project","title":"Workspace","session_id":"session","workspace_control":true}]},"subtabs":{"project_id":"project","session_id":"session","schema_version":2,"required_owner":"${"b".repeat(64)}","workspace_revision":$revision,"subtab_revision":2,"active_subtab_id":"${"a".repeat(64)}","tabs":[{"id":"${"a".repeat(64)}","name":"main","kind":"main","concurrency":1}],"subtab_control":true,"directory_tab_control":false,"worktree_control":false,"concurrency_control":false}}""")
        fun page(revision: Int, next: Int = 0, done: Boolean = false) = response(next, done).apply {
            getAsJsonObject("payload").getAsJsonObject("result").add("subscription_view", view(revision))
        }
        val reducer = SubscriptionReducer(initial, project)
        val published = mutableListOf<HostProjection>()
        val input = Fragmented(wire(page(4), event(), page(3, 1, true), page(5, 1, true)))
        val error = runCatching {
            ResponseReader.read(input, request(), ::validate) { batch ->
                published.add(reducer.accept(batch.frames, batch.result))
            }
        }.exceptionOrNull()
        assertTrue(error is zenpi.state.StaleProjectionException)
        assertEquals(1, published.size); assertEquals(published.single(), reducer.projection)
        assertEquals("4", reducer.projection.workspaceRevision)
        assertEquals("0", reducer.projection.eventSequence); assertTrue(reducer.projection.turns.isEmpty())
        assertFalse(reducer.done); assertEquals(page(5, 1, true), FrameCodec.read(input))
    }
    @Test fun unsolicitedEventAndMoreThan128FramesRefuseBeforePublication() = runBlocking {
        val control = JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"subscription","payload":{"type":"request","operation":"workspace_list","arguments":{}}}""").asJsonObject
        for ((request, bytes) in listOf(control to wire(event(), response()),
            request() to wire(*(0 until 129).map { event(it) }.toTypedArray(), response(129, true)))) {
            var calls = 0
            assertTrue(runCatching {
                ResponseReader.read(Fragmented(bytes), request, ::validate) { calls++ }
            }.isFailure)
            assertEquals(0, calls)
        }
    }
    @Test fun changedDisplaySurvivesHeartbeatsAndIncompleteLaterTranscript() = runBlocking {
        val view = JsonParser.parseString("""{"listing":{"mutations_available":false,"active_project_id":"project","required_owner":"${"b".repeat(64)}","workspace_revision":4,"projects":[{"id":"project","title":"Renamed workspace","session_id":"session","workspace_control":true}]},"subtabs":{"project_id":"project","session_id":"session","schema_version":2,"required_owner":"${"b".repeat(64)}","workspace_revision":4,"subtab_revision":2,"active_subtab_id":"${"a".repeat(64)}","tabs":[{"id":"${"a".repeat(64)}","name":"renamed","kind":"main","concurrency":1}],"subtab_control":true,"directory_tab_control":false,"worktree_control":false,"concurrency_control":false}}""")
        val changed = response().apply { getAsJsonObject("payload").getAsJsonObject("result").add("subscription_view", view) }
        val reducer = SubscriptionReducer(initial, project)
        var calls = 0
        assertTrue(runCatching {
            ResponseReader.read(Fragmented(wire(changed, response(), event())), request(), ::validate) { page ->
                reducer.accept(page.frames, page.result); calls++
            }
        }.isFailure)
        assertEquals(2, calls); assertEquals("Renamed workspace", reducer.projection.projects.single().title)
        assertEquals("renamed", reducer.projection.subtabs?.tabs?.single()?.name)
        assertEquals("4", reducer.projection.workspaceRevision); assertEquals("0", reducer.projection.eventSequence)
        assertTrue(reducer.projection.turns.isEmpty()); assertFalse(reducer.done)
    }
    @Test fun malformedCompletionAndCumulativeBudgetCannotPublishBadPage() = runBlocking {
        for (value in listOf(response().apply { getAsJsonObject("payload").getAsJsonObject("result").remove("subscription_done") },
            response().apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty("subscription_done", "true") })) {
            var calls = 0
            assertTrue(runCatching { ResponseReader.read(Fragmented(wire(value)), request(), ::validate) { calls++ } }.isFailure)
            assertEquals(0, calls)
        }
        val reducer = SubscriptionReducer(initial, project)
        var calls = 0
        val first = response(1).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty("padding", "x".repeat(90_000)) }
        val second = response(2, true).apply { getAsJsonObject("payload").getAsJsonObject("result").addProperty("padding", "x".repeat(95_000)) }
        assertTrue(runCatching {
            ResponseReader.read(Fragmented(wire(event(0, "x".repeat(12_000)), first, event(1), second)), request(), ::validate) { page ->
                reducer.accept(page.frames, page.result); calls++
            }
        }.isFailure)
        assertEquals(1, calls); assertEquals("1", reducer.projection.eventSequence)
        assertEquals(1, reducer.projection.turns.size); assertFalse(reducer.done)
    }
}
