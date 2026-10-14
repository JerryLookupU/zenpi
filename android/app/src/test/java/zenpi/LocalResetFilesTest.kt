package zenpi

import org.junit.Assert.*
import org.junit.Test
import zenpi.security.LocalResetFiles
import java.io.File
import java.nio.file.Files

class LocalResetFilesTest {
    private fun exercise(block: (File) -> Unit) {
        val root = Files.createTempDirectory("zenpi-reset-").toFile()
        try { block(File(root, "paired-host")) } finally { root.deleteRecursively() }
    }
    @Test fun intentSurvivesNewInstanceAndOnlyFinishesAfterAllFilesRemoved() = exercise { base ->
        val reset = LocalResetFiles(base)
        reset.requireAvailable(); reset.begin(); reset.begin()
        assertTrue(runCatching { LocalResetFiles(base).requireAvailable() }.isFailure)
        for (suffix in listOf("", ".bak", ".new")) {
            val file = File(base.path + suffix); file.writeText("ciphertext")
            assertTrue(runCatching { reset.finish() }.isFailure)
            assertTrue(File(base.path + ".reset").exists()); assertTrue(file.delete())
        }
        reset.finish(); reset.requireAvailable(); reset.finish()
    }
    @Test fun failedMarkerRemovalNeverReportsSuccess() = exercise { base ->
        val marker = File(base.path + ".reset")
        assertTrue(marker.mkdir()); File(marker, "obstacle").writeText("owned fixture")
        val reset = LocalResetFiles(base)
        assertTrue(runCatching { reset.requireAvailable() }.isFailure)
        assertTrue(runCatching { reset.finish() }.isFailure)
        assertTrue(marker.exists())
    }
    @Test fun failedMarkerCreationDoesNotPermitDestructiveWork() = exercise { base ->
        val reset = LocalResetFiles(File(base, "missing-parent"))
        assertTrue(runCatching { reset.begin() }.isFailure)
        assertFalse(File(base, "missing-parent.reset").exists())
    }
}
