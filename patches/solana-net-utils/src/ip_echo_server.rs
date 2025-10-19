//! IP echo server utilities with const function compatibility fix

use std::num::NonZeroUsize;

// Fixed: Use const compatible version for Rust 1.82
pub const MINIMUM_IP_ECHO_SERVER_THREADS_VALUE: usize = 2;

#[inline]
pub fn minimum_ip_echo_server_threads() -> NonZeroUsize {
    // Safety: 2 is always non-zero
    NonZeroUsize::new(MINIMUM_IP_ECHO_SERVER_THREADS_VALUE).unwrap()
}

// Legacy constant for compatibility
pub static MINIMUM_IP_ECHO_SERVER_THREADS: std::sync::LazyLock<NonZeroUsize> = 
    std::sync::LazyLock::new(|| minimum_ip_echo_server_threads());

// Add other exports that might be needed by the test framework
pub use std::net::{IpAddr, Ipv4Addr, Ipv6Addr, SocketAddr};

// Basic IP utilities
pub fn is_host(string: &str) -> bool {
    string.parse::<IpAddr>().is_ok()
}

pub fn parse_host(host: &str) -> Result<IpAddr, std::net::AddrParseError> {
    host.parse()
}

pub fn parse_port_or_addr(port_or_addr: &str) -> Result<u16, Box<dyn std::error::Error>> {
    if let Ok(port) = port_or_addr.parse::<u16>() {
        Ok(port)
    } else if let Ok(addr) = port_or_addr.parse::<SocketAddr>() {
        Ok(addr.port())
    } else {
        Err("Invalid port or address".into())
    }
}

// Add basic server functionality stubs
pub fn bind_common(
    _addr: SocketAddr,
    _port_range: std::ops::Range<u16>,
) -> Result<std::net::UdpSocket, std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Stub implementation",
    ))
}

pub fn bind_in_range(
    _addr: IpAddr,
    _port_range: std::ops::Range<u16>,
) -> Result<(u16, std::net::UdpSocket), std::io::Error> {
    Err(std::io::Error::new(
        std::io::ErrorKind::Unsupported,
        "Stub implementation",
    ))
}