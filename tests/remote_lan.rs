use std::net::IpAddr;
use zenpi::remote::lan::{local_addresses, select};

fn ip(value: &str) -> IpAddr {
    value.parse().unwrap()
}

#[test]
fn offline_lan_and_private_interface_win_without_internet_or_default_route() {
    for route in [
        None,
        Some(ip("203.0.113.9")),
        Some(ip("100.64.1.2")),
        Some(ip("127.0.0.1")),
    ] {
        assert_eq!(
            select(true, &[ip("192.168.50.55")], route).unwrap(),
            ip("192.168.50.55")
        );
    }
    assert_eq!(
        select(false, &[ip("fd00::1234")], None).unwrap(),
        ip("fd00::1234")
    );
}

#[test]
fn multiple_lans_require_a_matching_route_or_explicit_operator_selection() {
    let candidates = [ip("192.168.50.55"), ip("10.7.0.2")];
    for route in [
        None,
        Some(ip("203.0.113.9")),
        Some(ip("172.16.8.4")),
        Some(ip("fd00::1")),
    ] {
        assert!(
            select(true, &candidates, route)
                .unwrap_err()
                .contains("multiple LAN")
        );
    }
    for route in candidates {
        assert_eq!(select(true, &candidates, Some(route)).unwrap(), route);
    }
    assert!(select(false, &[ip("fd00::1"), ip("fd00::2")], None).is_err());
}

#[test]
fn unusable_or_other_family_addresses_cannot_become_a_qr_candidate() {
    let candidates = [
        "0.0.0.0",
        "127.0.0.1",
        "169.254.1.2",
        "224.0.0.1",
        "255.255.255.255",
        "::",
        "::1",
        "fe80::1",
        "ff02::1",
    ];
    for ipv4 in [true, false] {
        for candidate in candidates {
            assert!(select(ipv4, &[ip(candidate)], Some(ip(candidate))).is_err());
        }
    }
    assert!(select(true, &[ip("fd00::1")], Some(ip("fd00::1"))).is_err());
    assert!(select(false, &[ip("192.168.1.2")], Some(ip("192.168.1.2"))).is_err());
}

#[test]
fn mapped_ipv4_route_cannot_hide_loopback_link_local_broadcast_or_multicast() {
    for address in [
        "::ffff:0.0.0.0",
        "::ffff:127.0.0.1",
        "::ffff:127.9.8.7",
        "::ffff:169.254.1.2",
        "::ffff:224.0.0.1",
        "::ffff:255.255.255.255",
    ] {
        let address = ip(address);
        assert!(select(false, &[], Some(address)).is_err());
        assert!(select(false, &[address], Some(address)).is_err());
        assert_eq!(
            select(false, &[ip("fd00::1")], Some(address)).unwrap(),
            ip("fd00::1")
        );
    }
    let reachable = ip("::ffff:192.168.1.2");
    assert_eq!(select(false, &[], Some(reachable)).unwrap(), reachable);
    assert!(select(true, &[], Some(reachable)).is_err());
}

#[test]
fn interface_order_and_duplicates_do_not_choose_arbitrarily() {
    let lan = ip("172.16.0.9");
    assert_eq!(
        select(true, &[lan, lan, ip("127.0.0.1"), ip("fd00::1")], None).unwrap(),
        lan
    );
    let other = ip("192.168.1.4");
    assert!(select(true, &[lan, other], None).is_err());
    assert!(select(true, &[other, lan], None).is_err());
}

#[test]
fn no_private_interface_retains_a_valid_same_family_route() {
    assert_eq!(
        select(true, &[], Some(ip("203.0.113.9"))).unwrap(),
        ip("203.0.113.9")
    );
    assert_eq!(
        select(false, &[], Some(ip("2001:db8::9"))).unwrap(),
        ip("2001:db8::9")
    );
    assert!(
        select(true, &[], None)
            .unwrap_err()
            .contains("no usable LAN")
    );
}

#[test]
fn selected_offline_lan_address_roundtrips_through_the_actual_qr_invite() {
    use zenpi::remote::pairing::{HostIdentity, InviteIssuer, PairingInvite, unix_seconds};
    let identity = HostIdentity::generate().unwrap();
    let issuer = InviteIssuer::new(&identity);
    for address in [ip("192.168.50.55"), ip("fd00::1234")] {
        let selected = select(address.is_ipv4(), &[address], None).unwrap();
        let invite = issuer
            .issue(
                std::net::SocketAddr::new(selected, 7777),
                std::time::Duration::from_secs(60),
            )
            .unwrap();
        let decoded =
            PairingInvite::from_uri(&invite.to_uri().unwrap(), unix_seconds().unwrap()).unwrap();
        assert_eq!(decoded.address.ip(), address);
        assert_eq!(decoded.address.port(), 7777);
        assert_eq!(decoded, invite);
        assert!(!zenpi::host_pair::qr_lines(&invite).unwrap().is_empty());
    }
}

#[cfg(unix)]
#[test]
fn actual_interface_inventory_is_unique_and_excludes_unusable_addresses() {
    let addresses = local_addresses().unwrap();
    let mut sorted = addresses.clone();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(addresses, sorted);
    for address in addresses {
        assert!(!address.is_loopback());
        assert!(!address.is_unspecified());
        assert!(!address.is_multicast());
        match address {
            IpAddr::V4(ip) => {
                assert!(!ip.is_link_local());
                assert!(!ip.is_broadcast());
            }
            IpAddr::V6(ip) => assert!(!ip.is_unicast_link_local()),
        }
    }
}
