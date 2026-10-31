use std::net::{TcpStream, ToSocketAddrs};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PingStatus {
    Testing,
    Latency(u128),
    Timeout,
}

/// Measure TCP handshake latency to DoT port 853.
/// ponytail: measures TCP RTT; upgrade to full TLS handshake timing if cipher negotiation overhead matters.
#[must_use]
pub fn ping_dot(host: &str) -> Option<u128> {
    let start = Instant::now();
    let addrs = (host, 853).to_socket_addrs().ok()?;
    let addr = addrs.into_iter().next()?;
    TcpStream::connect_timeout(&addr, Duration::from_millis(2000)).ok()?;
    Some(start.elapsed().as_millis())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_invalid_host() {
        assert!(ping_dot("invalid.dns.test.local").is_none());
    }
}
