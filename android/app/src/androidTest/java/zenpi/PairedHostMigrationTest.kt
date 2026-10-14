package zenpi

import androidx.test.platform.app.InstrumentationRegistry
import org.junit.Assert.*
import org.junit.Test
import zenpi.security.*
import zenpi.transport.*

/** ZE-515: a host saved with a v1 certificate pin migrates to its key id in place. */
class PairedHostMigrationTest {
    @Test fun legacyCertificatePinMigratesToHostIdKeepingDisplayAndPendingRequest() {
        val context = InstrumentationRegistry.getInstrumentation().targetContext
        val store = PairedHostStore(context, "migration-${System.nanoTime()}")
        try {
            // A legacy record is exactly what the pre-ZE-515 writer produced: no pin kind.
            val legacy = PairedHost("192.168.50.55", 7777, "a".repeat(64), "b".repeat(64), legacyPin = true)
            store.save(legacy)
            assertEquals(legacy, store.load())
            val projection = HostProjection(listOf(RemoteProject("project", "Workspace", "session", workspaceControl = true)), "project",
                "session", "idle", listOf(RemoteTurn("t", "assistant", "kept")), "1", false, "1",
                workspaceOwner = "c".repeat(64), workspaceRevision = "1")
            store.saveProjection(legacy, projection)
            val command = WorkspaceCommand.create(projection, "project", "workspace_select")
            store.beginWorkspace(legacy, command)
            val hostId = "d".repeat(64)
            val migrated = store.migratePin(legacy, hostId)
            assertEquals(legacy.copy(fingerprint = hostId, legacyPin = false), migrated)
            val state = requireNotNull(store.loadState())
            assertEquals(migrated, state.host)
            assertEquals(projection.turns, state.projection?.turns)
            assertEquals(command.envelope, state.pendingWorkspace?.envelope)
            // Only a legacy host migrates, and only from the saved record.
            assertTrue(runCatching { store.migratePin(migrated, "e".repeat(64)) }.isFailure)
            assertTrue(runCatching { store.migratePin(legacy, "e".repeat(64)) }.isFailure)
            assertTrue(runCatching { store.migratePin(legacy.copy(port = 1), "e".repeat(64)) }.isFailure)
            assertEquals(migrated, store.load())
            // New pairings persist the key-id pin directly.
            store.finishWorkspace(migrated, command)
            store.save(migrated.copy(port = 8888))
            assertFalse(requireNotNull(store.load()).legacyPin)
            // ZE-516: the QR's later hints persist in order after the primary address.
            val overlay = migrated.copy(fallbacks = listOf(zenpi.pairing.AddressHint("tailnet", "100.101.102.103", 7777),
                zenpi.pairing.AddressHint("tailnet", "fd7a:115c:a1e0::1", 7777)))
            store.save(overlay)
            assertEquals(overlay, store.load())
            assertEquals(listOf("192.168.50.55:7777", "100.101.102.103:7777", "[fd7a:115c:a1e0::1]:7777"),
                requireNotNull(store.load()).hints.map { it.address })
            assertTrue(runCatching { store.save(overlay.copy(fallbacks = listOf(zenpi.pairing.AddressHint("tailnet", "192.168.50.55", 7777)))) }.isFailure)
        } finally { store.wipe() }
    }
}
