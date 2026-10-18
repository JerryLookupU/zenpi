package zenpi.security

import java.io.File
import java.io.FileOutputStream

internal class LocalResetFiles(private val base: File) {
    private val marker = File(base.path + ".reset")
    fun requireAvailable() { check(!marker.exists()) { "Local reset incomplete" } }
    fun begin() {
        if (marker.exists()) return
        FileOutputStream(marker).use { it.write(byteArrayOf(1)); it.fd.sync() }
        check(marker.exists()) { "Reset intent persistence failed" }
    }
    fun finish() {
        check(listOf(base, File(base.path + ".bak"), File(base.path + ".new")).none { it.exists() }) {
            "Metadata file removal failed"
        }
        check(!marker.exists() || marker.delete()) { "Reset intent removal failed" }
        check(!marker.exists()) { "Local reset incomplete" }
    }
}
