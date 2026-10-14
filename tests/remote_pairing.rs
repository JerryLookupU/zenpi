use std::{sync::Arc, time::Duration};
use zenpi::remote::pairing::{HostIdentity, InviteIssuer, PairingInvite, unix_seconds};

#[test]
fn identity_is_persistent_private_and_malformed_identity_is_not_replaced() {
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("identity.json");
    let first = HostIdentity::load_or_create(&path).unwrap();
    let second = HostIdentity::load_or_create(&path).unwrap();
    assert_eq!(first.fingerprint(), second.fingerprint());
    assert_eq!(first.certificate(), second.certificate());
    first.server_config().unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        assert_eq!(
            std::fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }
    std::fs::write(&path, "malformed").unwrap();
    assert!(HostIdentity::load_or_create(&path).is_err());
    assert_eq!(std::fs::read_to_string(&path).unwrap(), "malformed");
}

#[cfg(unix)]
#[test]
fn symlink_and_public_identity_are_refused() {
    use std::os::unix::fs::{PermissionsExt, symlink};
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("identity.json");
    HostIdentity::load_or_create(&path).unwrap();
    let link = directory.path().join("link.json");
    symlink(&path, &link).unwrap();
    assert!(HostIdentity::load_or_create(&link).is_err());
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o644)).unwrap();
    assert!(HostIdentity::load_or_create(&path).is_err());
}

#[cfg(unix)]
#[test]
fn identity_hardlink_directory_and_fifo_are_refused_without_replacement_or_blocking() {
    use std::{ffi::CString, os::unix::ffi::OsStrExt};
    const FIFO_CHILD: &str = "ZENPI_IDENTITY_FIFO_TEST";
    if let Some(path) = std::env::var_os(FIFO_CHILD) {
        assert!(HostIdentity::load_or_create(std::path::Path::new(&path)).is_err());
        return;
    }
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("identity.json");
    HostIdentity::load_or_create(&path).unwrap();
    let before = std::fs::read(&path).unwrap();
    let alias = directory.path().join("identity-alias.json");
    std::fs::hard_link(&path, &alias).unwrap();
    for candidate in [&path, &alias] {
        assert!(HostIdentity::load_or_create(candidate).is_err());
        assert_eq!(std::fs::read(candidate).unwrap(), before);
    }
    std::fs::remove_file(&alias).unwrap();
    assert!(HostIdentity::load_or_create(&path).is_ok());
    let folder = directory.path().join("identity-directory");
    std::fs::create_dir(&folder).unwrap();
    assert!(HostIdentity::load_or_create(&folder).is_err());
    assert!(folder.is_dir());

    let fifo = directory.path().join("identity-fifo");
    let native = CString::new(fifo.as_os_str().as_bytes()).unwrap();
    assert_eq!(unsafe { libc::mkfifo(native.as_ptr(), 0o600) }, 0);
    // Bound the child so a regression to blocking open cannot hang the suite.
    let started = std::time::Instant::now();
    let mut child = std::process::Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "identity_hardlink_directory_and_fifo_are_refused_without_replacement_or_blocking",
        ])
        .env(FIFO_CHILD, &fifo)
        .spawn()
        .unwrap();
    loop {
        if let Some(status) = child.try_wait().unwrap() {
            assert!(status.success());
            break;
        }
        if started.elapsed() >= Duration::from_secs(3) {
            child.kill().unwrap();
            child.wait().unwrap();
            panic!("identity FIFO load blocked without a writer");
        }
        std::thread::sleep(Duration::from_millis(10));
    }
    use std::os::unix::fs::FileTypeExt;
    assert!(
        std::fs::symlink_metadata(&fifo)
            .unwrap()
            .file_type()
            .is_fifo()
    );
    assert_eq!(std::fs::read(&path).unwrap(), before);
}

#[test]
fn invite_roundtrip_bounds_and_debug_redaction() {
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    let invite = issuer
        .issue(
            "192.168.50.55:7777".parse().unwrap(),
            Duration::from_secs(60),
        )
        .unwrap();
    let uri = invite.to_uri().unwrap();
    assert_eq!(
        PairingInvite::from_uri(&uri, unix_seconds().unwrap()).unwrap(),
        invite
    );
    assert!(!format!("{invite:?}").contains(&invite.token));
    for suffix in ["&v=1", "&credential=secret", "#fragment"] {
        assert!(PairingInvite::from_uri(&(uri.clone() + suffix), unix_seconds().unwrap()).is_err());
    }
    for bad in [
        uri.replace("zenpi://pair", "https://pair"),
        uri.replace("zenpi://pair", "zenpi://user@pair"),
        uri.replace("zenpi://pair", "zenpi://pair/path"),
        "x".repeat(2049),
    ] {
        assert!(PairingInvite::from_uri(&bad, unix_seconds().unwrap()).is_err());
    }
    assert!(PairingInvite::from_uri(&uri, invite.expires_at).is_err());
    assert!(PairingInvite::from_uri(&uri, invite.expires_at - 301).is_err());
    let mut noncanonical = invite.token.clone();
    noncanonical.push('=');
    assert!(
        PairingInvite::from_uri(
            &uri.replace(&invite.token, &noncanonical),
            unix_seconds().unwrap()
        )
        .is_err()
    );
}

#[test]
fn invites_are_one_use_and_refresh_cancel_and_wrong_proof_fail_closed() {
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    let first = issuer
        .issue("127.0.0.1:7777".parse().unwrap(), Duration::from_secs(60))
        .unwrap();
    assert!(issuer.consume("wrong", &first.fingerprint).is_err());
    assert!(issuer.consume(&first.token, &"a".repeat(64)).is_err());
    issuer.consume(&first.token, &first.fingerprint).unwrap();
    assert!(issuer.consume(&first.token, &first.fingerprint).is_err());
    let second = issuer
        .issue(first.address, Duration::from_secs(60))
        .unwrap();
    let third = issuer
        .issue(first.address, Duration::from_secs(60))
        .unwrap();
    assert_ne!(second.token, third.token);
    assert!(issuer.consume(&second.token, &second.fingerprint).is_err());
    issuer.cancel().unwrap();
    assert!(issuer.consume(&third.token, &third.fingerprint).is_err());
    for (address, ttl) in [
        ("0.0.0.0:5", 60),
        ("127.0.0.1:0", 60),
        ("224.0.0.1:5", 60),
        ("127.0.0.1:5", 0),
        ("127.0.0.1:5", 301),
    ] {
        assert!(
            issuer
                .issue(address.parse().unwrap(), Duration::from_secs(ttl))
                .is_err()
        );
    }
}

#[test]
fn concurrent_admission_has_exactly_one_winner() {
    let identity = HostIdentity::generate().unwrap();
    let issuer = Arc::new(InviteIssuer::new(&identity));
    let invite = issuer
        .issue("127.0.0.1:7777".parse().unwrap(), Duration::from_secs(60))
        .unwrap();
    let barrier = Arc::new(std::sync::Barrier::new(8));
    let threads: Vec<_> = (0..8)
        .map(|_| {
            let issuer = issuer.clone();
            let invite = invite.clone();
            let barrier = barrier.clone();
            std::thread::spawn(move || {
                barrier.wait();
                issuer.consume(&invite.token, &invite.fingerprint).is_ok()
            })
        })
        .collect();
    assert_eq!(
        threads
            .into_iter()
            .map(|task| usize::from(task.join().unwrap()))
            .sum::<usize>(),
        1
    );
}

#[test]
fn monotonic_and_wall_clock_expiry_are_enforced() {
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    let invite = issuer
        .issue("127.0.0.1:7777".parse().unwrap(), Duration::from_secs(1))
        .unwrap();
    std::thread::sleep(Duration::from_millis(1100));
    assert!(issuer.consume(&invite.token, &invite.fingerprint).is_err());
}

#[test]
fn shared_android_rust_qr_invitation_corpus_matches_address_and_refusal() {
    let mut names = std::collections::BTreeSet::new();
    for row in include_str!("../fuzz/remote_protocol/pairing-corpus.jsonl").lines() {
        let case: serde_json::Value = serde_json::from_str(row).unwrap();
        let name = case["name"].as_str().unwrap();
        assert!(names.insert(name.to_owned()));
        let invite =
            PairingInvite::from_uri(case["uri"].as_str().unwrap(), case["now"].as_u64().unwrap());
        assert_eq!(
            invite.is_ok(),
            case["accept"].as_bool().unwrap(),
            "{name}: {invite:?}"
        );
        if let Ok(invite) = invite {
            assert_eq!(
                invite.address.to_string(),
                case["address"].as_str().unwrap(),
                "{name}"
            );
            assert!(!format!("{invite:?}").contains(&invite.token));
        }
    }
    assert!(names.len() >= 40);
}

fn rotation_fixture() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../fuzz/remote_protocol/host-key-rotation.json"
    ))
    .unwrap()
}

/// ZE-515: `host_id = sha256(SPKI)`; a new certificate for the same key keeps it.
#[test]
fn host_id_is_the_key_hash_and_survives_certificate_rotation() {
    let fixture = rotation_fixture();
    let directory = tempfile::tempdir().unwrap();
    let mut loaded = vec![];
    for (index, record) in fixture["identities"].as_array().unwrap().iter().enumerate() {
        let path = directory.path().join(format!("identity-{index}.json"));
        let mut options = std::fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        use std::io::Write;
        options
            .open(&path)
            .unwrap()
            .write_all(&serde_json::to_vec(record).unwrap())
            .unwrap();
        loaded.push(HostIdentity::load_or_create(&path).unwrap());
    }
    assert_ne!(loaded[0].certificate(), loaded[1].certificate());
    for (index, identity) in loaded.iter().enumerate() {
        assert_eq!(identity.fingerprint(), fixture["host_id"]);
        assert_eq!(identity.legacy_fingerprint(), fixture["legacy_pins"][index]);
    }
    let generated = HostIdentity::generate().unwrap();
    let rotated = generated.rotate_certificate().unwrap();
    assert_eq!(rotated.fingerprint(), generated.fingerprint());
    assert_ne!(rotated.certificate(), generated.certificate());
    assert_ne!(rotated.legacy_fingerprint(), generated.legacy_fingerprint());
    rotated.server_config().unwrap();
    // The issuer pins the key, so an invite from either certificate is the same host.
    let invite = InviteIssuer::new(&rotated)
        .issue(
            "192.168.50.55:7777".parse().unwrap(),
            Duration::from_secs(60),
        )
        .unwrap();
    assert_eq!(invite.fingerprint, generated.fingerprint());
}

#[test]
fn issued_invites_are_v2_with_ordered_token_bound_hints_and_v1_still_parses() {
    use zenpi::remote::pairing::{AddressHint, HintKind};
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    let lan: std::net::SocketAddr = "192.168.50.55:7777".parse().unwrap();
    let tailnet: std::net::SocketAddr = "[fd7a:115c:a1e0::1]:7777".parse().unwrap();
    let hints = vec![
        AddressHint::lan(lan),
        AddressHint {
            kind: HintKind::Tailnet,
            address: tailnet,
        },
    ];
    let invite = issuer
        .issue_hints(hints.clone(), Duration::from_secs(60))
        .unwrap();
    assert_eq!(invite.schema_version, 2);
    assert_eq!(invite.address, lan);
    assert_eq!(invite.hints, hints);
    assert_eq!(invite.fingerprint, identity.fingerprint());
    let uri = invite.to_uri().unwrap();
    assert!(uri.starts_with("zenpi://pair?v=2&host_id="));
    assert!(!uri.contains("address=") && !uri.contains("fingerprint="));
    let now = unix_seconds().unwrap();
    assert_eq!(PairingInvite::from_uri(&uri, now).unwrap(), invite);
    // Hints live and die with the token: refresh replaces both, the old token is dead.
    let refreshed = issuer
        .issue_hints(invite.hints.clone(), Duration::from_secs(60))
        .unwrap();
    assert_eq!(refreshed.hints, hints);
    assert!(issuer.consume(&invite.token, &invite.fingerprint).is_err());
    issuer
        .consume(&refreshed.token, &refreshed.fingerprint)
        .unwrap();
    // v1 still parses strictly, as exactly one LAN hint.
    let v1 = format!(
        "zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint={}&token={}&expires={}",
        "a".repeat(64),
        invite.token,
        invite.expires_at
    );
    let parsed = PairingInvite::from_uri(&v1, now).unwrap();
    assert_eq!(parsed.schema_version, 1);
    assert_eq!(parsed.hints, vec![AddressHint::lan(lan)]);
    assert_eq!(
        PairingInvite::from_uri(&parsed.to_uri().unwrap(), now).unwrap(),
        parsed
    );
    let token = &invite.token;
    let expires = invite.expires_at;
    let host = identity.fingerprint();
    let one = "lan%3A192.168.50.55%3A7777";
    for bad in [
        // v1 and v2 field sets never mix.
        format!(
            "zenpi://pair?v=2&host_id={host}&hints={one}&address=192.168.50.55%3A7777&token={token}&expires={expires}"
        ),
        format!("zenpi://pair?v=2&fingerprint={host}&hints={one}&token={token}&expires={expires}"),
        format!(
            "zenpi://pair?v=1&address=192.168.50.55%3A7777&fingerprint={host}&hints={one}&token={token}&expires={expires}"
        ),
        format!("zenpi://pair?v=3&host_id={host}&hints={one}&token={token}&expires={expires}"),
        format!("zenpi://pair?v=2&host_id={host}&token={token}&expires={expires}"),
        format!("zenpi://pair?v=2&host_id={host}&hints=&token={token}&expires={expires}"),
        format!(
            "zenpi://pair?v=2&host_id={host}&hints=wan%3A192.168.50.55%3A7777&token={token}&expires={expires}"
        ),
        format!(
            "zenpi://pair?v=2&host_id={host}&hints={one}%2C{one}&token={token}&expires={expires}"
        ),
        format!(
            "zenpi://pair?v=2&host_id={host}&hints=lan%3A192.168.50.55%3A0&token={token}&expires={expires}"
        ),
        format!(
            "zenpi://pair?v=2&host_id={host}&hints=lan%3Aexample.com%3A7777&token={token}&expires={expires}"
        ),
        format!("zenpi://pair?v=2&host_id={host}&hints={one}%2C&token={token}&expires={expires}"),
        format!(
            "zenpi://pair?v=2&host_id={host}&hints=lan%3A10.0.0.1%3A1%2Clan%3A10.0.0.2%3A1%2Clan%3A10.0.0.3%3A1%2Clan%3A10.0.0.4%3A1%2Clan%3A10.0.0.5%3A1&token={token}&expires={expires}"
        ),
    ] {
        assert!(PairingInvite::from_uri(&bad, now).is_err(), "{bad}");
    }
    assert!(issuer.issue_hints(vec![], Duration::from_secs(60)).is_err());
}
