package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.pairing.*
import zenpi.security.PairedHost

class DeviceActionPolicyTest {
    private val state = PairingState(phase = PairingPhase.CONNECTED, registered = true,
        host = PairedHost("192.168.50.55", 7777, "a".repeat(64), "b".repeat(64)))

    @Test fun scanningAndResettingFenceDeviceActionsWithoutDroppingPendingIdentity() {
        for (phase in PairingPhase.values()) {
            for (blocked in listOf(state.copy(phase = phase, scanInProgress = true), state.copy(phase = phase, resetting = true))) {
                assertFalse(DeviceActionPolicy.revoke(blocked))
                assertFalse(DeviceActionPolicy.forget(blocked))
                assertFalse(DeviceActionPolicy.checkRevocation(blocked.copy(pendingRevoke = "exact-request")))
                assertEquals("exact-request", blocked.copy(pendingRevoke = "exact-request").pendingRevoke)
            }
        }
    }

    @Test fun offlineRevocationAndExplicitLocalResetRetainIndependentSemantics() {
        for (phase in listOf(PairingPhase.CONNECTED, PairingPhase.ERROR, PairingPhase.OFFLINE)) {
            val current = state.copy(phase = phase)
            assertTrue(DeviceActionPolicy.revoke(current))
            assertTrue(DeviceActionPolicy.forget(current))
            assertFalse(DeviceActionPolicy.checkRevocation(current))
            assertFalse(DeviceActionPolicy.revoke(current.copy(pendingRevoke = "pending")))
            assertTrue(DeviceActionPolicy.checkRevocation(current.copy(pendingRevoke = "pending")))
        }
        assertFalse(DeviceActionPolicy.revoke(state.copy(host = null)))
        assertFalse(DeviceActionPolicy.checkRevocation(state.copy(host = null, pendingRevoke = "pending")))
        assertFalse(DeviceActionPolicy.revoke(state.copy(resetRequired = true)))
        assertFalse(DeviceActionPolicy.checkRevocation(state.copy(resetRequired = true, pendingRevoke = "pending")))
        assertFalse(DeviceActionPolicy.forget(state.copy(phase = PairingPhase.CONNECTING)))
        assertTrue(DeviceActionPolicy.forget(PairingState(phase = PairingPhase.CONNECTING, resetRequired = true)))
    }
}
