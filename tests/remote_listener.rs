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
    let _first = TcpStream::connect(listener.local_addr()).unwrap();
    wait_for(|| listener.active_connections() == 1);
    let mut excess = TcpStream::connect(listener.local_addr()).unwrap();
    excess
        .set_read_timeout(Some(Duration::from_secs(1)))
        .unwrap();
    assert_eq!(excess.read(&mut [0; 1]).unwrap(), 0);
    assert!(listener.active_connections() <= 1);
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
