//! Local interface selection for a LAN-first pairing QR plus T1 overlay hints
//! (ZE-516); no discovery packets and no Tailscale API.

use std::net::IpAddr;

fn usable(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !ip.is_link_local()
                && !ip.is_broadcast()
        }
        IpAddr::V6(ip) => {
            if let Some(mapped) = ip.to_ipv4_mapped() {
                return usable(IpAddr::V4(mapped));
            }
            !ip.is_unspecified()
                && !ip.is_loopback()
                && !ip.is_multicast()
                && !ip.is_unicast_link_local()
        }
    }
}

fn private(ip: IpAddr) -> bool {
    !overlay(ip)
        && match ip {
            IpAddr::V4(ip) => ip.is_private(),
            IpAddr::V6(ip) => ip.is_unique_local(),
        }
}

/// A T1 overlay address: Tailscale/Headscale CGNAT `100.64.0.0/10` or the Tailscale
/// ULA `fd7a:115c:a1e0::/48`. Membership is decided from the address alone.
pub fn overlay(ip: IpAddr) -> bool {
    match ip {
        IpAddr::V4(ip) => {
            let [first, second, ..] = ip.octets();
            first == 100 && second & 0xc0 == 0x40
        }
        IpAddr::V6(ip) => match ip.to_ipv4_mapped() {
            Some(mapped) => overlay(IpAddr::V4(mapped)),
            None => ip.segments()[..3] == [0xfd7a, 0x115c, 0xa1e0],
        },
    }
}

/// This host's usable overlay addresses, IPv4 first. Overlay tunnels are usually
/// point-to-point interfaces, which LAN selection deliberately skips.
pub fn overlay_addresses() -> std::io::Result<Vec<IpAddr>> {
    let mut addresses: Vec<_> = interface_addresses(true)?
        .into_iter()
        .filter(|ip| overlay(*ip))
        .collect();
    addresses.sort_by_key(|ip| (ip.is_ipv6(), *ip));
    Ok(addresses)
}

/// Prefer a private interface over a public/VPN default route. When multiple
/// LANs remain, only a route naming one of those interfaces resolves ambiguity.
pub fn select(
    ipv4: bool,
    interfaces: &[IpAddr],
    route: Option<IpAddr>,
) -> Result<IpAddr, &'static str> {
    let mut candidates: Vec<_> = interfaces
        .iter()
        .copied()
        .filter(|ip| ip.is_ipv4() == ipv4 && usable(*ip) && private(*ip))
        .collect();
    candidates.sort_unstable();
    candidates.dedup();
    let route = route.filter(|ip| ip.is_ipv4() == ipv4 && usable(*ip));
    match candidates.as_slice() {
        [only] => Ok(*only),
        [] => route.ok_or("no usable LAN address; use --bind IP:PORT or --advertise IP"),
        _ => route
            .filter(|ip| candidates.contains(ip))
            .ok_or("multiple LAN addresses; use --bind IP:PORT or --advertise IP"),
    }
}

/// Up, non-loopback, non-point-to-point interface addresses (LAN candidates).
pub fn local_addresses() -> std::io::Result<Vec<IpAddr>> {
    interface_addresses(false)
}

#[cfg(unix)]
fn interface_addresses(point_to_point: bool) -> std::io::Result<Vec<IpAddr>> {
    struct Interfaces(*mut libc::ifaddrs);
    impl Drop for Interfaces {
        fn drop(&mut self) {
            // SAFETY: this guard owns only the list returned by getifaddrs.
            if !self.0.is_null() {
                unsafe { libc::freeifaddrs(self.0) };
            }
        }
    }
    let mut head = std::ptr::null_mut();
    // SAFETY: getifaddrs initializes the pointer on success; the guard frees it.
    if unsafe { libc::getifaddrs(&mut head) } != 0 {
        return Err(std::io::Error::last_os_error());
    }
    let list = Interfaces(head);
    let mut addresses = Vec::new();
    let mut cursor = list.0;
    while !cursor.is_null() {
        // SAFETY: cursor traverses the OS-owned list while its guard is alive.
        let entry = unsafe { &*cursor };
        if entry.ifa_flags & libc::IFF_UP as libc::c_uint != 0
            && entry.ifa_flags & libc::IFF_LOOPBACK as libc::c_uint == 0
            && (point_to_point || entry.ifa_flags & libc::IFF_POINTOPOINT as libc::c_uint == 0)
            && !entry.ifa_addr.is_null()
        {
            // SAFETY: non-null address belongs to this list; family determines its layout.
            let family = i32::from(unsafe { (*entry.ifa_addr).sa_family });
            let ip = match family {
                libc::AF_INET => {
                    // SAFETY: AF_INET identifies sockaddr_in on supported Unix hosts.
                    let socket = unsafe { &*entry.ifa_addr.cast::<libc::sockaddr_in>() };
                    Some(IpAddr::V4(std::net::Ipv4Addr::from(
                        socket.sin_addr.s_addr.to_ne_bytes(),
                    )))
                }
                libc::AF_INET6 => {
                    // SAFETY: AF_INET6 identifies sockaddr_in6 on supported Unix hosts.
                    let socket = unsafe { &*entry.ifa_addr.cast::<libc::sockaddr_in6>() };
                    (socket.sin6_scope_id == 0)
                        .then(|| IpAddr::V6(std::net::Ipv6Addr::from(socket.sin6_addr.s6_addr)))
                }
                _ => None,
            };
            if let Some(ip) = ip.filter(|ip| usable(*ip)) {
                addresses.push(ip);
            }
        }
        cursor = entry.ifa_next;
    }
    addresses.sort_unstable();
    addresses.dedup();
    Ok(addresses)
}

#[cfg(not(unix))]
fn interface_addresses(_point_to_point: bool) -> std::io::Result<Vec<IpAddr>> {
    Ok(Vec::new())
}
