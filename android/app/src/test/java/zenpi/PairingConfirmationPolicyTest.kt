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
    @Test fun exactAsciiCodeAndFreshInviteAreRequiredBeforeRegistration() {
        val invite = invite(); val state = PairingState(PairingPhase.CONFIRMING, invite)
        assertTrue(PairingConfirmationPolicy.allows(state, invite.safetyCode, 1000))
        assertTrue(PairingConfirmationPolicy.allows(state, invite.safetyCode, 1059))
        for (code in listOf("", "123", "1234567", " ${invite.safetyCode}", invite.safetyCode + "\n", "\u0661\u0662\u0663\u0664\u0665\u0666"))
            assertFalse(PairingConfirmationPolicy.allows(state, code, 1000))
        for (now in listOf(1060L, 1061L, 759L, -1L, Long.MIN_VALUE, Long.MAX_VALUE))
            assertFalse(PairingConfirmationPolicy.allows(state, invite.safetyCode, now))
    }
    @Test fun otherPhasesResetAndPendingAuthorityNeverPermitConfirmation() {
        val invite = invite(); val state = PairingState(PairingPhase.CONFIRMING, invite)
        val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true)),
            "project", "session", "idle", emptyList(), "1", false, "0", workspaceOwner = "b".repeat(64), workspaceRevision = "1")
        val command = WorkspaceCommand.create(projection, "project", "workspace_select")
        for (bad in PairingPhase.entries.filter { it != PairingPhase.CONFIRMING }.map { state.copy(phase = it) }
            + listOf(state.copy(invite = null), state.copy(scanInProgress = true), state.copy(resetting = true), state.copy(resetRequired = true),
                state.copy(pendingWorkspace = command), state.copy(pendingRevoke = "revoke")))
            assertFalse(PairingConfirmationPolicy.allows(bad, invite.safetyCode, 1000))
    }
    @Test fun replacementWithSameHostAndExpiryIsStillANewInvitation() {
        val old = invite(); val replacement = invite(1)
        assertEquals(old.fingerprint, replacement.fingerprint); assertEquals(old.expiresAt, replacement.expiresAt)
        assertNotEquals(old.safetyCode, replacement.safetyCode)
        val state = PairingState(PairingPhase.CONFIRMING, replacement)
        assertFalse(PairingConfirmationPolicy.allows(state, old.safetyCode, 1000))
        assertTrue(PairingConfirmationPolicy.allows(state, replacement.safetyCode, 1000))
    }
}
