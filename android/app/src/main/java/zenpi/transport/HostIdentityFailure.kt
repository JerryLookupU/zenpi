package zenpi.transport

import java.security.cert.CertificateException
import java.util.Collections
import java.util.IdentityHashMap

internal class HostIdentityException(cause: CertificateException? = null) :
    CertificateException("Host identity verification failed", cause)

/** TLS engines may wrap the trust-manager refusal; never classify by diagnostic text. */
internal object HostIdentityFailure {
    fun refused(error: Throwable): Boolean {
        val visited = Collections.newSetFromMap(IdentityHashMap<Throwable, Boolean>())
        var current: Throwable? = error
        while (current != null && visited.add(current)) {
            if (current is HostIdentityException) return true
            current = current.cause
        }
        return false
    }
}
