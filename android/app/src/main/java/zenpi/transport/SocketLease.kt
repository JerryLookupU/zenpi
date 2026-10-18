package zenpi.transport

import java.net.Socket
import java.net.SocketException

/** Cancellation is terminal, including while a raw socket is being wrapped in TLS. */
internal class SocketLease : AutoCloseable {
    private val lock = Any()
    private var socket: Socket? = null
    private var closed = false

    fun install(raw: Socket) {
        val accepted = synchronized(lock) {
            if (closed || socket != null) false else { socket = raw; true }
        }
        if (!accepted) reject(raw)
    }

    /** Release a raw socket whose connect attempt failed so the next hint may be installed. */
    fun retire(raw: Socket) {
        synchronized(lock) { if (socket === raw) socket = null }
        raw.close()
    }

    fun transfer(raw: Socket, tls: Socket) {
        val accepted = synchronized(lock) {
            if (closed || socket !== raw) false else { socket = tls; true }
        }
        // The TLS wrapper owns raw; closing raw here would also break a valid transfer.
        if (!accepted) reject(tls)
    }

    override fun close() {
        val owned = synchronized(lock) {
            closed = true
            socket.also { socket = null }
        }
        owned?.close()
    }

    private fun reject(candidate: Socket): Nothing {
        val error = SocketException("Connection ownership expired")
        try { candidate.close() } catch (closeError: Exception) { error.addSuppressed(closeError) }
        throw error
    }
}
