//! Network utilities for Solana

pub mod ip_echo_client;
pub mod ip_echo_server;

pub use ip_echo_client::*;
pub use ip_echo_server::*;

use std::net::{IpAddr, SocketAddr, UdpSocket};
use std::ops::Range;

// Socket configuration
#[derive(Debug, Clone)]
pub struct SocketConfig {
    pub reuseaddr: bool,
    pub reuse_port: bool,
}

impl Default for SocketConfig {
    fn default() -> Self {
        Self {
            reuseaddr: true,
            reuse_port: false,
        }
    }
}

// Port range for validators
pub const VALIDATOR_PORT_RANGE: Range<u16> = 8000..10000;

// Additional networking functions needed by the test framework
pub fn bind_to_unspecified() -> Result<UdpSocket, std::io::Error> {
    UdpSocket::bind("0.0.0.0:0")
}

pub fn bind_in_range_with_config(
    addr: IpAddr,
    port_range: Range<u16>,
    _config: SocketConfig,
) -> Result<(u16, UdpSocket), std::io::Error> {
    bind_in_range_internal(addr, port_range)
}

pub fn bind_with_any_port_with_config(
    addr: IpAddr,
    _config: SocketConfig,
) -> Result<UdpSocket, std::io::Error> {
    UdpSocket::bind(SocketAddr::new(addr, 0))
}

pub fn multi_bind_in_range(
    addr: IpAddr,
    port_range: Range<u16>,
    _num_sockets: usize,
) -> Result<Vec<(u16, UdpSocket)>, std::io::Error> {
    // For testing, just return one socket
    let (port, socket) = bind_in_range_internal(addr, port_range)?;
    Ok(vec![(port, socket)])
}

pub fn find_available_port_in_range(
    addr: IpAddr,
    port_range: Range<u16>,
) -> Result<u16, std::io::Error> {
    let (port, _) = bind_in_range_internal(addr, port_range)?;
    Ok(port)
}

// Implement bind_in_range properly - internal function to avoid naming conflict
fn bind_in_range_internal(addr: IpAddr, port_range: Range<u16>) -> Result<(u16, UdpSocket), std::io::Error> {
    for port in port_range {
        let socket_addr = SocketAddr::new(addr, port);
        if let Ok(socket) = UdpSocket::bind(socket_addr) {
            return Ok((port, socket));
        }
    }
    Err(std::io::Error::new(
        std::io::ErrorKind::AddrInUse,
        "No available ports in range",
    ))
}