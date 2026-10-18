#![cfg(feature = "remote")]

use std::{
    io::Read,
    net::TcpStream,
    sync::Arc,
    time::{Duration, Instant},
};
use zenpi::remote::{
    frame::{read_frame, write_frame},
    listener::{ListenerOptions, RemoteListener},
    pairing::HostIdentity,
    protocol::{Channel, Envelope, Message},
};

fn client(
    identity: &HostIdentity,
    address: std::net::SocketAddr,
) -> rustls::StreamOwned<rustls::ClientConnection, TcpStream> {
    let mut roots = rustls::RootCertStore::empty();
    roots
        .add(rustls::pki_types::CertificateDer::from(
            identity.certificate().to_vec(),
        ))
        .unwrap();
    let mut config = rustls::ClientConfig::builder_with_provider(Arc::new(
        rustls::crypto::ring::default_provider(),
    ))
    .with_protocol_versions(&[&rustls::version::TLS13])
    .unwrap()
    .with_root_certificates(roots)
    .with_no_client_auth();
    config.alpn_protocols = vec![b"zenpi/1".to_vec()];
    let connection =
        rustls::ClientConnection::new(Arc::new(config), "zenpi.local".try_into().unwrap()).unwrap();
    let socket = TcpStream::connect(address).unwrap();
    socket
        .set_read_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    socket
        .set_write_timeout(Some(Duration::from_secs(2)))
        .unwrap();
    rustls::StreamOwned::new(connection, socket)
}

fn wait_for(mut condition: impl FnMut() -> bool) {
    let deadline = Instant::now() + Duration::from_secs(3);
    while !condition() {
        assert!(Instant::now() < deadline, "condition timeout");
        std::thread::sleep(Duration::from_millis(10));
    }
}

#[test]
fn real_tls_frames_and_shutdown_release_listener() {
    let identity = HostIdentity::generate().unwrap();
    let mut listener = RemoteListener::start(
        ListenerOptions::loopback(),
        identity.server_config().unwrap(),
        |stream| {
            let envelope = read_frame(stream)
                .map_err(std::io::Error::other)?
                .ok_or_else(|| std::io::Error::other("EOF"))?;
            assert!(matches!(envelope.payload, Message::Hello { .. }));
            write_frame(
                stream,
                &Envelope::new(
                    Channel::Control,
                    Message::Welcome {
                        selected_version: 1,
                    },
                ),
            )
            .map_err(std::io::Error::other)
        },
    )
    .unwrap();
    let address = listener.local_addr();
    let mut stream = client(&identity, address);
    write_frame(
        &mut stream,
        &Envelope::new(
            Channel::Control,
            Message::Hello {
                supported_versions: vec![1],
            },
        ),
    )
    .unwrap();
    assert!(matches!(
        read_frame(&mut stream).unwrap().unwrap().payload,
        Message::Welcome {
            selected_version: 1
        }
    ));
    assert_eq!(
        stream.conn.protocol_version(),
        Some(rustls::ProtocolVersion::TLSv1_3)
    );
    drop(stream);
    wait_for(|| listener.active_connections() == 0);
    listener.shutdown().unwrap();
    listener.shutdown().unwrap();
    assert!(TcpStream::connect(address).is_err());
}

#[test]
fn certificate_mismatch_fails_before_handler() {
    let identity = HostIdentity::generate().unwrap();
    let wrong = HostIdentity::generate().unwrap();
    let calls = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let count = calls.clone();
    let mut listener = RemoteListener::start(
        ListenerOptions::loopback(),
        identity.server_config().unwrap(),
        move |_| {
            count.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    )
    .unwrap();
    let mut stream = client(&wrong, listener.local_addr());
    assert!(
        write_frame(
            &mut stream,
            &Envelope::new(
                Channel::Control,
                Message::Hello {
                    supported_versions: vec![1]
                }
            )
        )
        .is_err()
    );
    assert_eq!(calls.load(std::sync::atomic::Ordering::SeqCst), 0);
    listener.shutdown().unwrap();
}

#[test]
fn half_open_connections_are_capped_timed_out_and_shutdown_is_bounded() {
    let identity = HostIdentity::generate().unwrap();
    let mut options = ListenerOptions::loopback();
    options.max_connections = 1;
    options.handshake_timeout = Duration::from_millis(150);
    let mut listener = RemoteListener::start(options, identity.server_config().unwrap(), |_| {
        panic!("unauthenticated peer")
    })
    .unwrap();
    let mut first = TcpStream::connect(listener.local_addr()).unwrap();
    wait_for(|| listener.active_connections() == 1);
    // The same IP's newer half-open connection replaces the older one.
    let mut newer = TcpStream::connect(listener.local_addr()).unwrap();
    first
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    assert_eq!(first.read(&mut [0; 1]).unwrap(), 0);
    assert!(listener.active_connections() <= 1);
    newer
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    assert_eq!(newer.read(&mut [0; 1]).unwrap(), 0);
    wait_for(|| listener.active_connections() == 0);
    let _next = TcpStream::connect(listener.local_addr()).unwrap();
    wait_for(|| listener.active_connections() == 1);
    let started = Instant::now();
    listener.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(2));
    assert_eq!(listener.active_connections(), 0);
}

#[test]
fn four_live_tls_connections_refuse_a_fifth_and_release_capacity() {
    let identity = HostIdentity::generate().unwrap();
    let mut listener = RemoteListener::start(
        ListenerOptions::loopback(),
        identity.server_config().unwrap(),
        |stream| {
            let hello = read_frame(stream).map_err(std::io::Error::other)?.unwrap();
            assert!(matches!(hello.payload, Message::Hello { .. }));
            stream.sock.authenticated()?;
            write_frame(
                stream,
                &Envelope::new(
                    Channel::Control,
                    Message::Welcome {
                        selected_version: 1,
                    },
                ),
            )
            .map_err(std::io::Error::other)?;
            // Hold each established TLS connection until its peer or listener closes it.
            let _ = read_frame(stream);
            Ok(())
        },
    )
    .unwrap();
    assert_eq!(ListenerOptions::loopback().max_connections, 4);
    let hello = Envelope::new(
        Channel::Control,
        Message::Hello {
            supported_versions: vec![1],
        },
    );
    let mut peers = Vec::new();
    for _ in 0..4 {
        let mut peer = client(&identity, listener.local_addr());
        write_frame(&mut peer, &hello).unwrap();
        assert!(matches!(
            read_frame(&mut peer).unwrap().unwrap().payload,
            Message::Welcome {
                selected_version: 1
            }
        ));
        peers.push(peer);
    }
    assert_eq!(listener.active_connections(), 4);
    let mut excess = client(&identity, listener.local_addr());
    assert!(write_frame(&mut excess, &hello).is_err());
    assert_eq!(listener.active_connections(), 4);
    drop(peers.pop());
    wait_for(|| listener.active_connections() == 3);
    let mut replacement = client(&identity, listener.local_addr());
    write_frame(&mut replacement, &hello).unwrap();
    assert!(matches!(
        read_frame(&mut replacement).unwrap().unwrap().payload,
        Message::Welcome {
            selected_version: 1
        }
    ));
    assert_eq!(listener.active_connections(), 4);
    listener.shutdown().unwrap();
    assert_eq!(listener.active_connections(), 0);
}

#[test]
fn absolute_frame_deadline_and_policy_refusal() {
    let identity = HostIdentity::generate().unwrap();
    let elapsed = Arc::new(std::sync::atomic::AtomicBool::new(false));
    let result = elapsed.clone();
    let mut listener = RemoteListener::start(
        ListenerOptions::loopback(),
        identity.server_config().unwrap(),
        move |stream| {
            stream.sock.set_deadline(Duration::from_millis(50))?;
            let mut bytes = [0; 1];
            assert!(stream.read(&mut bytes).is_err());
            result.store(true, std::sync::atomic::Ordering::SeqCst);
            Ok(())
        },
    )
    .unwrap();
    let mut stream = client(&identity, listener.local_addr());
    while stream.conn.is_handshaking() {
        stream.conn.complete_io(&mut stream.sock).unwrap();
    }
    wait_for(|| elapsed.load(std::sync::atomic::Ordering::SeqCst));
    listener.shutdown().unwrap();
    let mut options = ListenerOptions::loopback();
    options.max_connections = 0;
    assert!(RemoteListener::start(options, identity.server_config().unwrap(), |_| Ok(())).is_err());
}

fn hello() -> Envelope {
    Envelope::new(
        Channel::Control,
        Message::Hello {
            supported_versions: vec![1],
        },
    )
}

/// Hello stands in for device authentication: it promotes the connection.
fn authenticating_listener(options: ListenerOptions, identity: &HostIdentity) -> RemoteListener {
    RemoteListener::start(options, identity.server_config().unwrap(), |stream| {
        let hello = read_frame(stream).map_err(std::io::Error::other)?.unwrap();
        assert!(matches!(hello.payload, Message::Hello { .. }));
        stream.sock.authenticated()?;
        stream.sock.set_deadline(Duration::from_secs(30))?;
        write_frame(
            stream,
            &Envelope::new(
                Channel::Control,
                Message::Welcome {
                    selected_version: 1,
                },
            ),
        )
        .map_err(std::io::Error::other)?;
        let _ = read_frame(stream);
        Ok(())
    })
    .unwrap()
}

fn authenticate(
    identity: &HostIdentity,
    address: std::net::SocketAddr,
) -> rustls::StreamOwned<rustls::ClientConnection, TcpStream> {
    let mut peer = client(identity, address);
    write_frame(&mut peer, &hello()).unwrap();
    assert!(matches!(
        read_frame(&mut peer).unwrap().unwrap().payload,
        Message::Welcome { .. }
    ));
    peer
}

#[test]
fn half_open_flood_keeps_one_unauthenticated_slot_and_authenticated_clients_connect() {
    let identity = HostIdentity::generate().unwrap();
    let mut listener = authenticating_listener(ListenerOptions::loopback(), &identity);
    let address = listener.local_addr();
    let established = authenticate(&identity, address);
    let mut half_open: Vec<TcpStream> = (0..12)
        .map(|_| TcpStream::connect(address).unwrap())
        .collect();
    // All but the newest half-open connection from this IP were replaced.
    for peer in &mut half_open[..11] {
        peer.set_read_timeout(Some(Duration::from_secs(2))).unwrap();
        assert_eq!(peer.read(&mut [0; 1]).unwrap(), 0);
    }
    wait_for(|| listener.active_connections() == 2);
    let late = authenticate(&identity, address);
    let also = authenticate(&identity, address);
    assert_eq!(listener.active_connections(), 3);
    drop((established, late, also, half_open));
    wait_for(|| listener.active_connections() == 0);
    listener.shutdown().unwrap();
}

#[test]
fn unauthenticated_budget_is_total_and_lifted_by_authentication() {
    let identity = HostIdentity::generate().unwrap();
    let mut listener = RemoteListener::start(
        options_with_budget(300),
        identity.server_config().unwrap(),
        |stream| {
            // A per-frame deadline cannot extend the unauthenticated budget.
            stream.sock.set_deadline(Duration::from_secs(30))?;
            let started = Instant::now();
            assert!(stream.read(&mut [0; 1]).is_err());
            assert!(started.elapsed() < Duration::from_secs(2));
            Ok(())
        },
    )
    .unwrap();
    let mut silent = client(&identity, listener.local_addr());
    while silent.conn.is_handshaking() {
        silent.conn.complete_io(&mut silent.sock).unwrap();
    }
    wait_for(|| listener.active_connections() == 0);
    listener.shutdown().unwrap();

    let mut listener = authenticating_listener(options_with_budget(300), &identity);
    let mut peer = client(&identity, listener.local_addr());
    write_frame(&mut peer, &hello()).unwrap();
    read_frame(&mut peer).unwrap().unwrap();
    std::thread::sleep(Duration::from_millis(600));
    assert_eq!(listener.active_connections(), 1);
    write_frame(&mut peer, &hello()).unwrap();
    wait_for(|| listener.active_connections() == 0);
    listener.shutdown().unwrap();
}

fn options_with_budget(millis: u64) -> ListenerOptions {
    let mut options = ListenerOptions::loopback();
    options.handshake_timeout = Duration::from_millis(millis);
    options
}

#[test]
fn idle_shutdown_wakes_the_blocking_accept() {
    let identity = HostIdentity::generate().unwrap();
    let mut listener = authenticating_listener(ListenerOptions::loopback(), &identity);
    std::thread::sleep(Duration::from_millis(50));
    let started = Instant::now();
    listener.shutdown().unwrap();
    assert!(started.elapsed() < Duration::from_secs(1));
}
