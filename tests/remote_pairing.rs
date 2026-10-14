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
