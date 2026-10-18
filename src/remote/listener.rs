//! Opt-in, bounded TLS listener with owned connection shutdown.

use std::{
    collections::BTreeMap,
    io::{self, Read, Write},
    net::{IpAddr, Ipv4Addr, Ipv6Addr, Shutdown, SocketAddr, TcpListener, TcpStream},
    sync::{
        Arc, Mutex,
        atomic::{AtomicBool, AtomicU64, Ordering},
    },
    thread::{self, JoinHandle},
    time::{Duration, Instant},
};

pub type TlsStream = rustls::StreamOwned<rustls::ServerConnection, DeadlineSocket>;

/// Absolute deadlines prevent a slow peer from extending a frame one byte at a time.
/// Until [`DeadlineSocket::authenticated`], every deadline is also capped by the
/// connection's total unauthenticated budget.
pub struct DeadlineSocket {
    socket: TcpStream,
    deadline: Instant,
    unauthenticated_until: Option<Instant>,
    state: Arc<State>,
    id: u64,
}

impl DeadlineSocket {
    pub fn set_deadline(&mut self, duration: Duration) -> io::Result<()> {
        if duration.is_zero() || duration > Duration::from_secs(300) {
            return Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "invalid remote deadline",
            ));
        }
        let deadline = Instant::now() + duration;
        self.deadline = self
            .unauthenticated_until
            .map_or(deadline, |budget| budget.min(deadline));
        Ok(())
    }

    /// The handler verified the peer's device key: lift the unauthenticated
    /// budget and free the peer's unauthenticated slot. Fails once replaced.
    pub fn authenticated(&mut self) -> io::Result<()> {
        let mut slots = self
            .state
            .slots
            .lock()
            .map_err(|_| io::Error::other("listener state poisoned"))?;
        slots
            .get_mut(&self.id)
            .ok_or_else(|| io::Error::other("remote connection replaced"))?
            .peer = None;
        self.unauthenticated_until = None;
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

impl Drop for DeadlineSocket {
    fn drop(&mut self) {
        if let Ok(mut slots) = self.state.slots.lock() {
            slots.remove(&self.id);
        }
    }
}

#[derive(Clone)]
pub struct ListenerOptions {
    pub bind: SocketAddr,
    pub max_connections: usize,
    /// Total budget for the unauthenticated phase (TLS handshake through
    /// device authentication), not a per-frame timeout.
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

struct Slot {
    socket: TcpStream,
    /// The peer's address while it is unauthenticated, `None` afterwards.
    peer: Option<IpAddr>,
}

#[derive(Default)]
struct State {
    stopping: AtomicBool,
    next_id: AtomicU64,
    slots: Mutex<BTreeMap<u64, Slot>>,
}

impl State {
    fn close_all(&self) {
        if let Ok(slots) = self.slots.lock() {
            for slot in slots.values() {
                let _ = slot.socket.shutdown(Shutdown::Both);
            }
        }
    }
}

/// Each IP holds at most one unauthenticated slot: a newer connection replaces
/// its older half-open one. Unauthenticated peers never occupy the half of the
/// slots reserved for authenticated connections.
fn admit(slots: &mut BTreeMap<u64, Slot>, peer: IpAddr, max: usize) -> bool {
    let previous = slots
        .iter()
        .find(|(_, slot)| slot.peer == Some(peer))
        .map(|(&id, _)| id);
    if let Some(previous) = previous.and_then(|id| slots.remove(&id)) {
        let _ = previous.socket.shutdown(Shutdown::Both);
    }
    let unauthenticated = slots.values().filter(|slot| slot.peer.is_some()).count();
    slots.len() < max && unauthenticated < max - max / 2
}

pub struct RemoteListener {
    address: SocketAddr,
    state: Arc<State>,
    accept_thread: Option<JoinHandle<io::Result<()>>>,
}

impl RemoteListener {
    /// Handler runs only after a real TLS 1.3 handshake. It must perform pairing
    /// or device authentication before dispatch; TLS alone is not authorization.
    /// Until it calls [`DeadlineSocket::authenticated`] the connection keeps
    /// its unauthenticated budget and slot.
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
        let state = Arc::new(State::default());
        let shared = state.clone();
        let handler = Arc::new(handler);
        let accept_thread = thread::Builder::new()
            .name("zenpi-remote-listener".into())
            .spawn(move || {
                let mut connections: Vec<JoinHandle<()>> = Vec::new();
                let result = (|| -> io::Result<()> {
                    // Blocking accept: an idle listener never wakes. Shutdown connects to wake it.
                    loop {
                        let accepted = listener.accept();
                        if shared.stopping.load(Ordering::Acquire) {
                            return Ok(());
                        }
                        connections.retain(|task| !task.is_finished());
                        let (socket, peer) = match accepted {
                            Ok(value) => value,
                            Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                            Err(error) => return Err(error),
                        };
                        let peer = peer.ip().to_canonical();
                        let mut slots = shared
                            .slots
                            .lock()
                            .map_err(|_| io::Error::other("listener state poisoned"))?;
                        if !admit(&mut slots, peer, options.max_connections) {
                            let _ = socket.shutdown(Shutdown::Both);
                            continue;
                        }
                        socket.set_nodelay(true)?;
                        let id = shared.next_id.fetch_add(1, Ordering::Relaxed);
                        slots.insert(
                            id,
                            Slot {
                                socket: socket.try_clone()?,
                                peer: Some(peer),
                            },
                        );
                        drop(slots);
                        let budget = Instant::now() + options.handshake_timeout;
                        let socket = DeadlineSocket {
                            socket,
                            deadline: budget,
                            unauthenticated_until: Some(budget),
                            state: shared.clone(),
                            id,
                        };
                        let config = config.clone();
                        let handler = handler.clone();
                        let task = thread::Builder::new()
                            .name("zenpi-remote-connection".into())
                            .spawn(move || {
                                let Ok(connection) = rustls::ServerConnection::new(config) else {
                                    return;
                                };
                                let mut stream = rustls::StreamOwned::new(connection, socket);
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
                })();
                shared.stopping.store(true, Ordering::Release);
                shared.close_all();
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
            .slots
            .lock()
            .map(|slots| slots.len())
            .unwrap_or(0)
    }

    pub fn shutdown(&mut self) -> io::Result<()> {
        self.state.stopping.store(true, Ordering::Release);
        self.state.close_all();
        if let Some(task) = self.accept_thread.take() {
            if task.is_finished() {
                return task
                    .join()
                    .map_err(|_| io::Error::other("remote listener panicked"))?;
            }
            // Wake the blocking accept so it observes `stopping`. If the address is
            // gone (interface down), leave the thread blocked rather than hang here;
            // every connection was already closed above.
            let mut wake = self.address;
            if wake.ip().is_unspecified() {
                wake.set_ip(match wake.ip() {
                    IpAddr::V4(_) => Ipv4Addr::LOCALHOST.into(),
                    IpAddr::V6(_) => Ipv6Addr::LOCALHOST.into(),
                });
            }
            if TcpStream::connect_timeout(&wake, Duration::from_secs(1)).is_err() {
                return Ok(());
            }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unauthenticated_peers_never_take_reserved_slots() {
        let local = TcpListener::bind("127.0.0.1:0").unwrap();
        let socket = || TcpStream::connect(local.local_addr().unwrap()).unwrap();
        let ip = |last: u8| IpAddr::from([10, 0, 0, last]);
        let mut slots = BTreeMap::new();
        for (id, last) in [1, 2].into_iter().enumerate() {
            assert!(admit(&mut slots, ip(last), 4));
            let peer = Some(ip(last));
            slots.insert(
                id as u64,
                Slot {
                    socket: socket(),
                    peer,
                },
            );
        }
        // Two distinct half-open peers fill the unauthenticated half of four slots.
        assert!(!admit(&mut slots, ip(3), 4));
        // A same-IP retry replaces its own half-open slot instead.
        assert!(admit(&mut slots, ip(2), 4));
        assert_eq!(slots.len(), 1);
        slots.get_mut(&0).unwrap().peer = None;
        assert!(admit(&mut slots, ip(3), 4));
    }
}
