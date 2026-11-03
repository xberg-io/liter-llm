//! Decoding of IPv6 addresses that carry an IPv4 address, shared by every outbound address check.

use std::net::{Ipv4Addr, Ipv6Addr};

/// The ISATAP interface identifier (RFC 5214) sits in bits 64 to 95: `0000:5efe` or `0200:5efe`.
pub(crate) fn is_isatap(ip: Ipv6Addr) -> bool {
    let segments = ip.segments();
    (segments[4] == 0x0000 || segments[4] == 0x0200) && segments[5] == 0x5efe
}

/// The IPv4 address that an IPv4-mapped, 6to4, Teredo, NAT64, SIIT or IPv4-compatible address carries.
pub(crate) fn embedded_ipv4(ip: Ipv6Addr) -> Option<Ipv4Addr> {
    let s = ip.segments();
    let low = |high: u16, low: u16| Ipv4Addr::from((u32::from(high) << 16) | u32::from(low));
    if let Some(mapped) = ip.to_ipv4_mapped() {
        return Some(mapped);
    }
    if s[0] == 0x2002 {
        return Some(low(s[1], s[2]));
    }
    if s[0] == 0x2001 && s[1] == 0 {
        // ~keep RFC 4380: the Teredo client address is the last 32 bits, inverted.
        return Some(low(!s[6], !s[7]));
    }
    let nat64 = s[0] == 0x0064 && s[1] == 0xff9b && s[2..6].iter().all(|&g| g == 0);
    let siit = s[0..4].iter().all(|&g| g == 0) && s[4] == 0xffff && s[5] == 0;
    if nat64 || siit {
        return Some(low(s[6], s[7]));
    }
    // ~keep IPv4-compatible `::a.b.c.d`, less `::` and `::1`, which are IPv6 addresses of their own.
    let carried = low(s[6], s[7]);
    (s[0..6].iter().all(|&g| g == 0) && u32::from(carried) > 1).then_some(carried)
}
