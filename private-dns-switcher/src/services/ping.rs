use std::net::{SocketAddr, TcpStream};
use std::time::{Duration, Instant};

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum PingStatus {
    Testing,
    Latency(u128),
    Timeout,
}

/// Measure TCP handshake latency to DoT port 853.
///
/// Uses the static provider table in `tunnel::resolve_dns_host` to avoid a blocking
/// DNS lookup (which would otherwise hang behind our own VPN tunnel).
/// ponytail: measures TCP RTT; upgrade to full TLS handshake timing if cipher negotiation overhead matters.
#[must_use]
pub fn ping_dot(host: &str) -> Option<u128> {
    let start = Instant::now();
    let addr = SocketAddr::new(crate::services::tunnel::resolve_dns_host(host)?.ip(), 853);
    TcpStream::connect_timeout(&addr, Duration::from_millis(2000)).ok()?;
    Some(start.elapsed().as_millis())
}

/// Run the blocking [`ping_dot`] on a dedicated thread so the Dioxus UI thread never stalls.
pub async fn ping_dot_async(host: String) -> Option<u128> {
    let (tx, rx) = futures_channel::oneshot::channel();
    std::thread::spawn(move || {
        let _ = tx.send(ping_dot(&host));
    });
    rx.await.ok().flatten()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ping_invalid_host() {
        assert!(ping_dot("invalid.dns.test.local").is_none());
    }
}
