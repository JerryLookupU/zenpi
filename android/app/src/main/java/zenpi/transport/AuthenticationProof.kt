package zenpi.transport

import kotlinx.coroutines.CancellationException

/** Bind one connection to its saved device, including key changes during TLS/signing. */
internal object AuthenticationProof {
    fun <T : Any> existingIdentity(expected: String, load: () -> T?, read: (T) -> String): T = local {
        val identity = load() ?: throw HostAuthenticationException()
        deviceId(expected) { read(identity) }
        identity
    }

    fun deviceId(expected: String, read: () -> String): String = local {
        read().also {
            if (!expected.matches(Regex("[0-9a-f]{64}")) || it != expected) throw HostAuthenticationException()
        }
    }

    fun current(expected: String, read: () -> String, checkActive: () -> Unit) {
        checkActive()
        deviceId(expected, read)
        // A blocking Keystore read can outlive foreground or network ownership.
        checkActive()
    }

    fun sign(expected: String, read: () -> String, checkActive: () -> Unit, sign: () -> String): String {
        current(expected, read, checkActive)
        val proof = local(sign)
        current(expected, read, checkActive)
        return proof
    }

    private fun <T> local(block: () -> T): T = try { block() }
    catch (cancelled: CancellationException) { throw cancelled }
    catch (_: Exception) { throw HostAuthenticationException() }
}
