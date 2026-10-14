//! Opt-in, bounded TLS listener with owned connection shutdown.

use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    net::{Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub type TlsStream = rustls::StreamOwned<rustls::ServerConnection, DeadlineSocket>;

/// Absolute deadlines prevent a slow peer from extending a frame one byte at a time.
pub struct DeadlineSocket {
    socket: TcpStream,
    deadline: Instant,
}

impl DeadlineSocket {
    pub fn set_deadline(&mut self, duration: Duration) -> io::Result<()> {
        if duration.is_zero() || duration > Duration::from_secs(300) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid remote deadline",
            ));
        }
        self.deadline = Instant::now() + duration;
        Ok(())
    }

    fn remaining(&self) -> io::Result<Duration> {
        self.deadline
            .checked_duration_since(Instant::now())
            .filter(|remaining| !remaining.is_zero())
            .ok_or_else(|| io::Error::new(io::ErrorKind::TimedOut, "remote deadline elapsed"))
    }
}

impl Read for DeadlineSocket {
    fn read(&mut self, bytes: &mut [u8]) -> io::Result<usize> {
        self.socket.set_read_timeout(Some(self.remaining()?))?;
        self.socket.read(bytes)
    }
}

impl Write for DeadlineSocket {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        self.socket.set_write_timeout(Some(self.remaining()?))?;
        self.socket.write(bytes)
    }
    fn flush(&mut self) -> io::Result<()> {
        self.remaining()?;
        self.socket.flush()
    }
}

#[derive(Clone)]
pub struct ListenerOptions {
    pub bind: SocketAddr,
    pub max_connections: usize,
    pub handshake_timeout: Duration,
}

impl ListenerOptions {
    pub fn loopback() -> Self {
        Self {
            bind: "127.0.0.1:0".parse().expect("loopback"),
            max_connections: 4,
            handshake_timeout: Duration::from_secs(5),
        }
    }

    fn validate(&self) -> io::Result<()> {
        if self.max_connections == 0
            || self.max_connections > 16
            || self.handshake_timeout.is_zero()
            || self.handshake_timeout > Duration::from_secs(30)
            || self.bind.ip().is_multicast()
        {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid remote listener policy",
            ));
        }
        Ok(())
    }
}

#[derive(Default)]
struct State {
    stopping: AtomicBool,
    next_id: AtomicU64,
    sockets: Mutex<BTreeMap<u64, TcpStream>>,
}

struct ConnectionGuard {
    state: Arc<State>,
    id: u64,
}

impl Drop for ConnectionGuard {
    fn drop(&mut self) {
        if let Ok(mut sockets) = self.state.sockets.lock() {
            sockets.remove(&self.id);
        }
    }
}

pub struct RemoteListener {
    address: SocketAddr,
    state: Arc<State>,
    accept_thread: Option<JoinHandle<io::Result<()>>>,
}

impl RemoteListener {
    /// Handler runs only after a real TLS 1.3 handshake. It must perform pairing
    /// or device authentication before dispatch; TLS alone is not authorization.
    pub fn start<F>(
        options: ListenerOptions,
        config: Arc<rustls::ServerConfig>,
        handler: F,
    ) -> io::Result<Self>
    where
        F: Fn(&mut TlsStream) -> io::Result<()> + Send + Sync + 'static,
    {
        options.validate()?;
        let listener = TcpListener::bind(options.bind)?;
        let address = listener.local_addr()?;
        listener.set_nonblocking(true)?;
        let state = Arc::new(State::default());
        let shared = state.clone();
        let handler = Arc::new(handler);
        let accept_thread = thread::Builder::new()
            .name("zenpi-remote-listener".into())
            .spawn(move || {
                let mut connections: Vec<JoinHandle<()>> = Vec::new();
                let result = (|| -> io::Result<()> {
                    while !shared.stopping.load(Ordering::Acquire) {
                        let mut index = 0;
                        while index < connections.len() {
                            if connections[index].is_finished() {
                                let _ = connections.swap_remove(index).join();
                            } else {
                                index += 1;
                            }
                        }
                        let (socket, _) = match listener.accept() {
                            Ok(value) => value,
                            Err(error) if error.kind() == io::ErrorKind::WouldBlock => {
                                thread::sleep(Duration::from_millis(10));
                                continue;
                            }
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            Err(error) => return Err(error),
                        };
                        let mut sockets = shared
                            .sockets
                            .lock()
                            .map_err(|_| io::Error::other("listener state poisoned"))?;
                        if shared.stopping.load(Ordering::Acquire)
                            || sockets.len() >= options.max_connections
                        {
                            let _ = socket.shutdown(Shutdown::Both);
                            continue;
                        }
                        // macOS accepted sockets inherit the nonblocking listener flag.
                        socket.set_nonblocking(false)?;
                        socket.set_nodelay(true)?;
                        let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
                        sockets.insert(id, socket.try_clone()?);
                        drop(sockets);
                        let guard = ConnectionGuard {
                            state: shared.clone(),
                            id,
                        };
                        let config = config.clone();
                        let handler = handler.clone();
                        let timeout = options.handshake_timeout;
                        let task = thread::Builder::new()
                            .name("zenpi-remote-connection".into())
                            .spawn(move || {
                                let _guard = guard;
                                let Ok(connection) = rustls::ServerConnection::new(config) else {
                                    return;
                                };
                                let mut stream = rustls::StreamOwned::new(
                                    connection,
                                    DeadlineSocket {
                                        socket,
                                        deadline: Instant::now() + timeout,
                                    },
                                );
                                while stream.conn.is_handshaking() {
                                    if stream.conn.complete_io(&mut stream.sock).is_err() {
                                        return;
                                    }
                                }
                                if stream.conn.alpn_protocol() != Some(b"zenpi/1".as_slice())
                                    || stream.conn.protocol_version()
                                        != Some(rustls::ProtocolVersion::TLSv1_3)
                                {
                                    return;
                                }
                                if stream.sock.set_deadline(Duration::from_secs(30)).is_err() {
                                    return;
                                }
                                let _ = handler(&mut stream);
                                stream.conn.send_close_notify();
                                let _ = stream.flush();
                            })?;
                        connections.push(task);
                    }
                    Ok(())
                })();
                shared.stopping.store(true, Ordering::Release);
                if let Ok(sockets) = shared.sockets.lock() {
                    for socket in sockets.values() {
                        let _ = socket.shutdown(Shutdown::Both);
                    }
                }
                for task in connections {
                    let _ = task.join();
                }
                result
            })?;
        Ok(Self {
            address,
            state,
            accept_thread: Some(accept_thread),
        })
    }

    pub fn local_addr(&self) -> SocketAddr {
        self.address
    }

    pub fn active_connections(&self) -> usize {
        self.state
            .sockets
            .lock()
            .map(|sockets| sockets.len())
            .unwrap_or(0)
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.state.stopping.store(true, Ordering::Release);
        if let Ok(sockets) = self.state.sockets.lock() {
            for socket in sockets.values() {
                let _ = socket.shutdown(Shutdown::Both);
            }
        }
        if let Some(task) = self.accept_thread.take() {
            task.join()
                .map_err(|_| io::Error::other("remote listener panicked"))??;
        }
        Ok(())
    }
}

impl Drop for RemoteListener {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}
