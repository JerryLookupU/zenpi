package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.transport.*
import java.util.Base64

class PairingConfirmationPolicyTest {
    private fun invite(byte: Byte = 0): PairingInvite {
        val token = Base64.getUrlEncoder().withoutPadding().encodeToString(ByteArray(32) { byte })
        return PairingInvite.parse("zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint=${"a".repeat(64)}&token=$token&expires=1060", 1000) { 0 }
    }
    @Test fun freshInviteIsRequiredBeforeRegistration() {
        val invite = invite(); val state = PairingState(PairingPhase.CONFIRMING, invite)
        assertTrue(PairingConfirmationPolicy.allows(state, 1000))
        assertTrue(PairingConfirmationPolicy.allows(state, 1059))
        for (now in listOf(1060L, 1061L, 759L, -1L, Long.MIN_VALUE, Long.MAX_VALUE))
            assertFalse(PairingConfirmationPolicy.allows(state, now))
    }
    @Test fun otherPhasesResetAndPendingAuthorityNeverPermitConfirmation() {
        val invite = invite(); val state = PairingState(PairingPhase.CONFIRMING, invite)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.create(projection, "project", "workspace_select")
        for (bad in PairingPhase.entries.filter { it != PairingPhase.CONFIRMING }.map { state.copy(phase = it) }
            + listOf(state.copy(invite = null), state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
                state.copy(pendingWorkspace = command), state.copy(pendingRevoke = "revoke")))
            assertFalse(PairingConfirmationPolicy.allows(bad, 1000))
    }
    @Test fun replacementWithSameHostAndExpiryIsStillANewInvitation() {
        val old = invite(); val replacement = invite(1)
        assertEquals(old.fingerprint, replacement.fingerprint); assertEquals(old.expiresAt, replacement.expiresAt)
        assertNotEquals(old.token, replacement.token)
        val state = PairingState(PairingPhase.CONFIRMING, replacement)
        assertTrue(PairingConfirmationPolicy.allows(state, 1000))
    }
}
