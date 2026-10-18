package zenpi

import com.google.gson.JsonObject
import com.google.gson.JsonParser
import kotlinx.coroutines.runBlocking
import org.junit.Assert.*
import org.junit.Test
import zenpi.transport.*
import zenpi.state.ProjectionCacheCodec

/** Production Rust adapter frames, not a Kotlin-built approximation of the host. */
class RustAndroidWireTest {
    private fun fixture(): JsonObject {
        val path = System.getProperty("zenpi.interop.fixture")
        val bytes = if (path != null) java.io.File(path).readBytes() else
            requireNotNull(javaClass.classLoader?.getResourceAsStream("android-sync-wire.json")).use { it.readBytes() }
        require(bytes.size <= 1024 * 1024)
        return JsonParser.parseString(bytes.toString(Charsets.UTF_8)).asJsonObject.also {
            assertEquals("stage2-rust-android-wire/v1", it["schema_version"].asString)
            assertTrue(it["fixture_only"].asBoolean)
        }
    }
    private fun exchange(fixture: JsonObject, id: String) = fixture["exchanges"].asJsonArray.single {
        it.asJsonObject["request"].asJsonObject["id"].asString == id
    }.asJsonObject
    private fun wire(exchange: JsonObject) = java.util.Base64.getDecoder().decode(exchange["wire_base64"].asString)
    private suspend fun read(exchange: JsonObject, publish: (suspend (ResponsePage) -> Unit)? = null): ResponsePage =
        ResponseReader.read(object : java.io.ByteArrayInputStream(wire(exchange)) {
            override fun read(bytes: ByteArray, offset: Int, length: Int): Int = super.read(bytes, offset, minOf(length, 3))
        }, exchange["request"].asJsonObject, {
            assertEquals("succeeded", FrameCodec.string(it, "execution_state"))
            assertFalse(it.has("code"))
        }, publish)
    private suspend fun initial(fixture: JsonObject): HostProjection {
        val projects = HostProjectionCodec.projects(read(exchange(fixture, "listing")).result)
        val selected = projects.single { it.id == fixture["project_id"].asString }
        return HostProjectionCodec.snapshot(projects, selected, read(exchange(fixture, "initial")).result)
    }

    @Test fun actualRustResumeUpdatesTheSameSessionAndCacheWithoutLeakingPrivateContent() = runBlocking {
        val fixture = fixture(); val initial = initial(fixture)
        assertEquals("1", initial.eventSequence); assertEquals("desktop seed", initial.turns.single().content)
        assertTrue(TranscriptRestorePolicy.replay(initial, initial.projects.single()))
        for (invalid in listOf(initial.copy(eventSequence = "01"), initial.copy(journalSequence = "0"),
            initial.copy(projects = initial.projects + initial.projects.single().copy(sessionId = "foreign")))) {
            assertFalse(TranscriptRestorePolicy.replay(invalid, initial.projects.single()))
        }
        val page = read(exchange(fixture, "resume"))
        val next = HostProjectionCodec.replay(initial, initial.projects, initial.projects.single(), page.frames, page.result)
        assertEquals(fixture["session_id"].asString, next.sessionId); assertEquals("2", next.eventSequence)
        assertEquals(1, next.replayedEventCount); assertFalse(next.hasMoreEvents)
        assertTrue(next.turns.last().content.startsWith("live desktop answer"))
        assertFalse(next.turns.last().content.contains("INTEROP_PRIVATE_BODY"))
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(next))
        assertEquals(next.turns, cached.turns); assertEquals(next.eventSequence, cached.eventSequence)
        assertNull(cached.workspaceOwner); assertFalse(cached.projects.single().promptControl)
    }

    @Test fun actualRustSubscriptionPublishesContiguousPagesAndMatchingTerminalState() = runBlocking {
        val fixture = fixture(); val initial = initial(fixture)
        val reducer = SubscriptionReducer(initial, initial.projects.single())
        var pages = 0
        read(exchange(fixture, "subscribe")) { page ->
            reducer.accept(page.frames, page.result); pages++
        }
        assertTrue(pages >= 2); assertTrue(reducer.done)
        assertEquals("2", reducer.projection.eventSequence); assertEquals(1, reducer.projection.replayedEventCount)
        assertEquals(2, reducer.projection.turns.size)
        assertEquals(initial.sessionId, reducer.projection.sessionId)
    }

    @Test fun currentRustOwnerRaceAuthorityRefusalsCannotTriggerSessionRecoveryOrPublication() = runBlocking {
        org.junit.Assume.assumeTrue(System.getProperty("zenpi.interop.fixture") != null)
        val fixture = fixture(); val initial = initial(fixture)
        val state = zenpi.pairing.PairingState(projection = initial)
        val failures = fixture["owner_race_failures"].asJsonArray
        assertEquals(setOf("unauthorized", "revoked_device"), failures.map { it.asJsonObject["code"].asString }.toSet())
        val deliveryFailures = fixture["authorization_failures"].asJsonArray
        assertEquals(2, deliveryFailures.size())
        assertEquals(setOf(false, true), deliveryFailures.map { it.asJsonObject["revoked"].asBoolean }.toSet())
        for (value in failures.toList() + deliveryFailures.toList()) {
            val exchange = value.asJsonObject
            val request = exchange["request"].asJsonObject
            var publications = 0
            var recoveries = 0
            val error = runCatching {
                ResponseReader.read(java.io.ByteArrayInputStream(wire(exchange)), request, { response ->
                    assertEquals("failed", FrameCodec.string(response, "execution_state"))
                    SessionChangedException.fromSubscription(request, response, true)?.let { recoveries++; throw it }
                    throw HostRefusedException(FrameCodec.string(response, "code"))
                }) { publications++ }
            }.exceptionOrNull()
            assertTrue(error is HostRefusedException)
            assertEquals(exchange["code"].asString, (error as HostRefusedException).code)
            assertTrue(zenpi.pairing.AuthorityLossPolicy.refused(error))
            assertEquals(0, recoveries); assertEquals(0, publications)
            val fenced = requireNotNull(zenpi.pairing.AuthorityLossPolicy.displayOnly(state).projection)
            assertEquals(initial.turns, fenced.turns); assertEquals(initial.eventSequence, fenced.eventSequence)
            assertNull(fenced.workspaceOwner)
            assertTrue(fenced.projects.all { !it.workspaceControl && !it.promptControl && !it.approvalControl })
        }
    }

    @Test fun currentRustEventFollowedByRevocationCannotCommitTranscriptOrCursor() = runBlocking {
        org.junit.Assume.assumeTrue(System.getProperty("zenpi.interop.fixture") != null)
        val fixture = fixture(); val initial = initial(fixture)
        val exchange = fixture["event_then_refusal"].asJsonObject
        val request = exchange["request"].asJsonObject
        val bytes = wire(exchange)
        val frames = java.io.ByteArrayInputStream(bytes)
        val event = requireNotNull(FrameCodec.read(frames))
        assertEquals("event", FrameCodec.string(event["payload"].asJsonObject, "type"))
        assertEquals(initial.eventSequence, event["sequence"].toString())
        assertEquals("unauthorized", FrameCodec.string(requireNotNull(FrameCodec.read(frames)), "code"))
        assertNull(FrameCodec.read(frames))
        val reducer = SubscriptionReducer(initial, initial.projects.single())
        val cached = ProjectionCacheCodec.encode(initial)
        var durable = cached.deepCopy()
        var visible = initial
        var publications = 0
        val input = object : java.io.ByteArrayInputStream(bytes) {
            override fun read(bytes: ByteArray, offset: Int, length: Int): Int = super.read(bytes, offset, minOf(length, 3))
        }
        val error = runCatching {
            ResponseReader.read(input, request, { response ->
                assertEquals("failed", FrameCodec.string(response, "execution_state"))
                assertNull(SessionChangedException.fromSubscription(request, response, true))
                throw HostRefusedException(FrameCodec.string(response, "code"))
            }) { page ->
                publications++
                visible = reducer.accept(page.frames, page.result)
                durable = ProjectionCacheCodec.encode(visible)
            }
        }.exceptionOrNull()
        assertTrue(error is HostRefusedException)
        assertEquals(0, publications); assertFalse(reducer.done)
        assertEquals(initial, reducer.projection); assertEquals(initial, visible); assertEquals(cached, durable)
        assertTrue(zenpi.pairing.AuthorityLossPolicy.refused(error as Exception))
        val fenced = requireNotNull(zenpi.pairing.AuthorityLossPolicy.displayOnly(zenpi.pairing.PairingState(projection = visible)).projection)
        assertEquals(initial.turns, fenced.turns); assertEquals(initial.eventSequence, fenced.eventSequence)
        assertEquals(cached, durable)
    }

    @Test fun missingCursorOnAnActualRustSnapshotCannotEraseTheSavedResumePosition() = runBlocking {
        val fixture = fixture(); val saved = initial(fixture)
        val snapshot = read(exchange(fixture, "initial")).result.deepCopy().apply {
            remove("event_cursor"); remove("event_cursor_scope")
        }
        val legacy = HostProjectionCodec.snapshot(saved.projects, saved.projects.single(), snapshot)
        assertNull(legacy.eventSequence)
        zenpi.state.ProjectionCommitPolicy.requireCurrent(null, legacy)
        val cached = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(saved))
        var durable = cached
        var visible = saved
        val error = runCatching {
            zenpi.state.ProjectionCommitPolicy.requireCurrent(durable, legacy)
            durable = ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(legacy))
            visible = legacy
        }.exceptionOrNull()
        assertTrue(error is WireException)
        assertEquals("Missing cache event sequence", error?.message)
        assertEquals(cached, durable); assertEquals(saved, visible)
        assertEquals("1", durable.eventSequence)
    }

    @Test fun currentRustBusySubscriptionPreservesTranscriptAndRejectsContradictoryAuthority() = runBlocking {
        org.junit.Assume.assumeTrue(System.getProperty("zenpi.interop.fixture") != null)
        val fixture = fixture(); val initial = initial(fixture)
        val reducer = SubscriptionReducer(initial, initial.projects.single())
        var pages = 0
        read(exchange(fixture, "busy-subscribe")) { page ->
            assertTrue(page.frames.isEmpty())
            assertFalse(page.result["subscription_transcript"].asBoolean)
            assertFalse(page.result.has("phase")); assertFalse(page.result.has("journal_next_sequence"))
            for (field in listOf("phase", "journal_next_sequence")) {
                val prior = reducer.projection
                val altered = page.result.deepCopy().apply {
                    if (field == "phase") addProperty(field, "running") else addProperty(field, 0)
                }
                assertTrue(runCatching { reducer.accept(page.frames, altered) }.exceptionOrNull() is WireException)
                assertEquals(prior, reducer.projection); assertFalse(reducer.done)
            }
            val next = reducer.accept(page.frames, page.result)
            assertEquals(initial.turns, next.turns); assertEquals(initial.phase, next.phase)
            assertEquals(initial.eventSequence, next.eventSequence); assertEquals(initial.journalSequence, next.journalSequence)
            pages++
        }
        assertTrue(pages >= 2); assertTrue(reducer.done)
    }

    @Test fun actualRustGapReplacesDisplayAndTruncatedWireCannotPublishIt() = runBlocking {
        val fixture = fixture(); val initial = initial(fixture)
        val resumed = read(exchange(fixture, "resume")).let {
            HostProjectionCodec.replay(initial, initial.projects, initial.projects.single(), it.frames, it.result)
        }
        val gap = exchange(fixture, "gap"); val page = read(gap)
        assertTrue(page.result["snapshot_required"].asBoolean)
        val next = HostProjectionCodec.replay(resumed, resumed.projects, resumed.projects.single(), page.frames, page.result)
        assertTrue(next.snapshotReplaced); assertEquals("302", next.eventSequence)
        assertEquals(64, next.turns.size); assertTrue(next.truncated)
        assertEquals("desktop history 299", next.turns.last().content)
        assertFalse(next.turns.any { it.id == "seed" })
        assertEquals(next.turns, ProjectionCacheCodec.decode(ProjectionCacheCodec.encode(next)).turns)
        var published = false
        val bytes = wire(gap)
        val failure = runCatching {
            val incomplete = ResponseReader.read(java.io.ByteArrayInputStream(bytes.copyOf(bytes.size - 1)), gap["request"].asJsonObject, {})
            HostProjectionCodec.replay(resumed, resumed.projects, resumed.projects.single(), incomplete.frames, incomplete.result)
            published = true
        }.exceptionOrNull()
        assertTrue(failure is java.io.EOFException); assertFalse(published)
    }

    @Test fun alteredActualRustReplayScopeCursorOrSnapshotCannotReplaceTheDisplay() = runBlocking {
        val fixture = fixture(); val initial = initial(fixture)
        val replay = read(exchange(fixture, "resume"))
        val resumed = HostProjectionCodec.replay(initial, initial.projects, initial.projects.single(), replay.frames, replay.result)
        val gap = read(exchange(fixture, "gap"))
        for (change in listOf<(MutableList<JsonObject>, JsonObject) -> Unit>(
            { frames, _ -> frames[0].addProperty("project_id", "foreign") },
            { frames, _ -> frames[0].addProperty("session_id", "foreign") },
            { _, result -> result.getAsJsonObject("cursor").addProperty("next_sequence", 303) },
            { frames, _ -> frames += frames[0].deepCopy() },
            { _, result -> result.addProperty("journal_next_sequence", 0) })) {
            val frames = gap.frames.map { it.deepCopy() }.toMutableList(); val result = gap.result.deepCopy()
            change(frames, result)
            var display = resumed
            assertTrue(runCatching {
                display = HostProjectionCodec.replay(resumed, resumed.projects, resumed.projects.single(), frames, result)
            }.exceptionOrNull() is WireException)
            assertEquals(resumed, display); assertEquals("2", display.eventSequence)
        }
    }
}
