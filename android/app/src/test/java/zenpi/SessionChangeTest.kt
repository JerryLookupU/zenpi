package zenpi

import com.google.gson.JsonObject
import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.PairedHost
import zenpi.transport.*

class SessionChangeTest {
    private val project = RemoteProject("project", "Workspace", "old", sessionControl = true)
    private val projection = HostProjection(listOf(project), "project", "old", "idle", listOf(RemoteTurn("turn", "assistant", "old display")), "5", false, "1")
    private val state = PairingState(phase = PairingPhase.CONNECTED, registered = true,
        host = PairedHost("192.168.1.2",7777,"a".repeat(64),"b".repeat(64)), projection = projection,
        sessionsOpen = true, terminalOpen = true, filesOpen = true, resourcesOpen = true, transcriptTurn = "turn")
    private fun request(type: String = "subscribe") = JsonObject().apply {
        addProperty("id","subscription"); addProperty("project_id","project"); addProperty("session_id","old")
        add("payload",JsonObject().apply {addProperty("type",type)})
    }
    private fun response(code: String = "session_changed") = JsonObject().apply {
        addProperty("id","subscription"); addProperty("project_id","project"); addProperty("code",code); addProperty("execution_state","failed")
    }
    @Test fun onlyCorrelatedReadOnlySubscriptionMayRequestFreshSession() {
        val error = requireNotNull(SessionChangedException.fromSubscription(request(),response(),true))
        assertEquals("project",error.projectId); assertEquals("old",error.previousSessionId)
        assertNull(SessionChangedException.fromSubscription(request("resume"),response(),true))
        assertNull(SessionChangedException.fromSubscription(request(),response(),false))
        for (code in listOf("owner_mismatch","unauthorized","revoked_device","busy"))
            assertNull(SessionChangedException.fromSubscription(request(),response(code),true))
        for (field in listOf("id","project_id")) assertTrue(runCatching {
            SessionChangedException.fromSubscription(request(),response().apply {addProperty(field,"foreign")},true)
        }.isFailure)
    }
    @Test fun transitionDropsOldCursorAndEveryTransientViewWithoutGrantingAuthority() {
        val error = SessionChangedException("project","old")
        assertTrue(SessionChangePolicy.allows(state,error,true,true))
        val next = SessionChangePolicy.replacement(state)
        assertNull(next.projection); assertEquals(state.host,next.host); assertTrue(next.registered)
        assertEquals(PairingPhase.CONNECTING,next.phase)
        assertFalse(next.sessionsOpen); assertFalse(next.terminalOpen); assertFalse(next.filesOpen); assertFalse(next.resourcesOpen)
        assertNull(next.transcriptTurn); assertNull(next.pendingWorkspace)
        assertFalse(AuthorityLossPolicy.refused(error))
    }
    @Test fun pendingWritesRevokesOfflineBackgroundAndWrongScopeNeverAutoReattach() {
        val error = SessionChangedException("project","old")
        assertFalse(SessionChangePolicy.allows(state,error,false,true))
        assertFalse(SessionChangePolicy.allows(state,error,true,false))
        for (changed in listOf(state.copy(pendingRevoke="pending"),state.copy(scanInProgress=true),state.copy(resetting=true),state.copy(resetRequired=true),
            state.copy(projection=projection.copy(projects=listOf(project,project.copy(sessionId="foreign")))),
            state.copy(projection=projection.copy(projects=emptyList())),
            state.copy(host=null),state.copy(projection=null),state.copy(projection=projection.copy(sessionId="new"))))
            assertFalse(SessionChangePolicy.allows(changed,error,true,true))
        assertFalse(SessionChangePolicy.allows(state,SessionChangedException("foreign","old"),true,true))
        assertFalse(SessionChangePolicy.allows(state,HostRefusedException("session_changed"),true,true))
        val pending = WorkspaceCommand.restore(com.google.gson.JsonParser.parseString("""{"schema_version":1,"channel":"control","id":"write","project_id":"project","session_id":"old","required_owner":"${"c".repeat(64)}","expected_revision":0,"payload":{"type":"request","operation":"session_new","arguments":{}}}""").asJsonObject)
        assertFalse(SessionChangePolicy.allows(state.copy(pendingWorkspace=pending),error,true,true))
    }
    @Test fun failedSubscriptionNeverPublishesItsUnconfirmedFrames() = kotlinx.coroutines.runBlocking {
        val req = request()
        val failure = response().apply {
            addProperty("schema_version",1);addProperty("channel","control")
            add("payload",JsonObject().apply {addProperty("type","response");add("result",com.google.gson.JsonNull.INSTANCE)})
        }
        val bytes = java.io.ByteArrayOutputStream().also {FrameCodec.write(it,failure)}.toByteArray()
        var publications = 0
        val error = runCatching {ResponseReader.read(java.io.ByteArrayInputStream(bytes),req, {value ->
            SessionChangedException.fromSubscription(req,value,true)?.let {throw it}
        }) {publications++}}.exceptionOrNull()
        assertTrue(error is SessionChangedException);assertEquals(0,publications)
    }
    @Test fun sessionRecoveryKeepsFailureBudgetUntilAFreshProjectionIsPublished() {
        val backoff = zenpi.lifecycle.ReconnectBackoff { it - 1 }
        repeat(3) { backoff.failed() }
        val changed = SessionChangedException("project", "old")
        assertTrue(SessionChangePolicy.allows(state, changed, true, true))
        val recovering = SessionChangePolicy.replacement(state)
        backoff.failed()
        assertEquals(8_000L, backoff.delayMillis(false))
        // An unconfirmed second switch cannot discard another cursor or restart recovery.
        assertFalse(SessionChangePolicy.allows(recovering, changed, true, true))
        backoff.failed()
        assertEquals(16_000L, backoff.delayMillis(false))
        backoff.succeeded()
        assertEquals(200L, backoff.delayMillis(true))
        assertEquals(2_000L, backoff.delayMillis(false))
    }
}
