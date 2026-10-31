use std::net::{IpAddr, Ipv4Addr, SocketAddr, TcpStream, ToSocketAddrs};
use std::time::Duration;

/// Compute RFC 791 IPv4 header checksum (16-bit one's complement sum).
#[must_use]
pub fn ip_checksum(data: &[u8]) -> u16 {
    let mut sum: u32 = 0;
    let (chunks, remainder) = data.as_chunks::<2>();
    for chunk in chunks {
        let word = u16::from_be_bytes(*chunk);
        sum = sum.wrapping_add(u32::from(word));
    }
    if let Some(&rem) = remainder.first() {
        let word = u16::from_be_bytes([rem, 0]);
        sum = sum.wrapping_add(u32::from(word));
    }
    while (sum >> 16) > 0 {
        sum = (sum & 0xFFFF) + (sum >> 16);
    }
    !(sum as u16)
}

/// Assemble an RFC 791 IPv4 header and RFC 768 UDP packet wrapping `payload`.
#[must_use]
pub fn build_ipv4_udp_packet(
    src_ip: [u8; 4],
    dst_ip: [u8; 4],
    src_port: u16,
    dst_port: u16,
    payload: &[u8],
) -> Vec<u8> {
    let total_len = 20 + 8 + payload.len();
    let mut pkt = Vec::with_capacity(total_len);

    // IPv4 Header (20 bytes)
    pkt.push(0x45); // Version 4, IHL 5
    pkt.push(0x00); // DSCP / ECN
    pkt.extend_from_slice(&(total_len as u16).to_be_bytes());
    pkt.extend_from_slice(&0u16.to_be_bytes()); // Identification
    pkt.extend_from_slice(&0x4000u16.to_be_bytes()); // Flags: DF (Don't Fragment)
    pkt.push(64); // TTL
    pkt.push(17); // Protocol: UDP
    pkt.extend_from_slice(&[0, 0]); // Checksum placeholder
    pkt.extend_from_slice(&src_ip);
    pkt.extend_from_slice(&dst_ip);

    // Calculate and place IPv4 header checksum
    let csum = ip_checksum(&pkt[0..20]);
    pkt[10..12].copy_from_slice(&csum.to_be_bytes());

    // UDP Header (8 bytes)
    let udp_len = (8 + payload.len()) as u16;
    pkt.extend_from_slice(&src_port.to_be_bytes());
    pkt.extend_from_slice(&dst_port.to_be_bytes());
    pkt.extend_from_slice(&udp_len.to_be_bytes());
    pkt.extend_from_slice(&[0, 0]); // UDP checksum optional in IPv4 (0 = none)

    // Payload
    pkt.extend_from_slice(payload);
    pkt
}

/// Resolve a DNS host or IP to a SocketAddr on port 53.
#[must_use]
pub fn resolve_dns_host(host: &str) -> Option<SocketAddr> {
    if let Ok(addr) = host.parse::<SocketAddr>() {
        return Some(addr);
    }
    if let Ok(ip) = host.parse::<IpAddr>() {
        return Some(SocketAddr::new(ip, 53));
    }

    // Fast static lookup for common public DNS providers
    let fallback_ip = match host.to_ascii_lowercase().as_str() {
        "dns.quad9.net" | "quad9" => Some(IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9))),
        "dns.google" | "google" => Some(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8))),
        "cloudflare-dns.com"
        | "1dot1dot1dot1.cloudflare-dns.com"
        | "one.one.one.one"
        | "cloudflare" => Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1))),
        "family.cloudflare-dns.com" => Some(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 3))),
        "dns.adguard.com" | "dns.adguard-dns.com" | "adguard" => {
            Some(IpAddr::V4(Ipv4Addr::new(94, 140, 14, 14)))
        }
        _ => None,
    };

    if let Some(ip) = fallback_ip {
        return Some(SocketAddr::new(ip, 53));
    }

    (host, 53).to_socket_addrs().ok()?.next()
}

/// Forward a raw DNS query to `upstream` over TCP (RFC 7766) and return the raw response.
///
/// Many ISPs transparently hijack UDP/53 answers (DNS poisoning), but leave TCP/53 alone.
/// Using TCP gives uncensored answers without needing TLS.
/// ponytail: unencrypted TCP/53. Upgrade to DoT (853) / DoH (443) if an ISP starts
/// hijacking TCP/53 as well; that requires a TLS stack (rustls).
#[must_use]
pub fn dns_over_tcp(upstream: SocketAddr, query: &[u8], timeout: Duration) -> Option<Vec<u8>> {
    use std::io::{Read, Write};

    if query.len() > u16::MAX as usize {
        return None;
    }

    let mut sock = TcpStream::connect_timeout(&upstream, timeout).ok()?;
    sock.set_read_timeout(Some(timeout)).ok()?;
    sock.set_write_timeout(Some(timeout)).ok()?;

    // RFC 7766: 2-byte big-endian length prefix, then the DNS message.
    let mut framed = Vec::with_capacity(query.len() + 2);
    framed.extend_from_slice(&(query.len() as u16).to_be_bytes());
    framed.extend_from_slice(query);
    sock.write_all(&framed).ok()?;

    let mut len_buf = [0u8; 2];
    sock.read_exact(&mut len_buf).ok()?;
    let mut resp = vec![0u8; u16::from_be_bytes(len_buf) as usize];
    sock.read_exact(&mut resp).ok()?;
    Some(resp)
}

#[cfg(target_os = "android")]
mod android_tunnel {
    use super::*;
    use std::sync::atomic::{AtomicBool, Ordering};

    static RUNNING: AtomicBool = AtomicBool::new(false);

    pub fn stop_tunnel() {
        RUNNING.store(false, Ordering::SeqCst);
    }

    pub fn run_tunnel(tun_fd: i32, dns_host: &str) {
        RUNNING.store(true, Ordering::SeqCst);

        let Some(upstream) = resolve_dns_host(dns_host) else {
            return;
        };

        // Only DNS (10.111.222.2/32) is routed into this tunnel, so upstream TCP
        // traffic egresses the default network directly — no protect() needed.
        let mut poll_fds = [libc::pollfd {
            fd: tun_fd,
            events: libc::POLLIN,
            revents: 0,
        }];
        let mut tun_buf = [0u8; 4096];

        while RUNNING.load(Ordering::Relaxed) {
            let ret = unsafe { libc::poll(poll_fds.as_mut_ptr(), 1, 250) };
            if ret < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }
            if ret == 0 || (poll_fds[0].revents & libc::POLLIN) == 0 {
                if (poll_fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                    break;
                }
                continue;
            }

            let n = unsafe {
                libc::read(
                    tun_fd,
                    tun_buf.as_mut_ptr() as *mut libc::c_void,
                    tun_buf.len(),
                )
            };
            if n <= 0 {
                break;
            }
            let n = n as usize;

            if !(n >= 28 && (tun_buf[0] >> 4) == 4) {
                continue;
            }
            let ihl = (tun_buf[0] & 0x0F) as usize * 4;
            if !(ihl >= 20 && n >= ihl + 8 && tun_buf[9] == 17) {
                continue;
            }
            let udp = &tun_buf[ihl..ihl + 8];
            if u16::from_be_bytes([udp[2], udp[3]]) != 53 {
                continue;
            }
            let client_port = u16::from_be_bytes([udp[0], udp[1]]);
            let payload_len = (u16::from_be_bytes([udp[4], udp[5]]) as usize).saturating_sub(8);
            if payload_len < 12 || ihl + 8 + payload_len > n {
                continue;
            }

            let client_ip = [tun_buf[12], tun_buf[13], tun_buf[14], tun_buf[15]];
            let virtual_dns_ip = [tun_buf[16], tun_buf[17], tun_buf[18], tun_buf[19]];
            let query = tun_buf[ihl + 8..ihl + 8 + payload_len].to_vec();

            // Resolve on a worker thread so slow upstreams never stall the tunnel loop.
            std::thread::spawn(move || {
                if let Some(resp) = dns_over_tcp(upstream, &query, Duration::from_secs(5)) {
                    let reply =
                        build_ipv4_udp_packet(virtual_dns_ip, client_ip, 53, client_port, &resp);
                    let _ = unsafe {
                        libc::write(tun_fd, reply.as_ptr() as *const libc::c_void, reply.len())
                    };
                }
            });
        }
    }
}

/// JNI entry point to run the DNS forwarding tunnel loop on Android.
///
/// # Safety
///
/// Must be called with valid JNI pointers from Android runtime on a dedicated thread.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_com_sicloud_dnsswitcher_DnsVpnService_nativeRunTunnel(
    env: *mut jni::sys::JNIEnv,
    this: jni::sys::jobject,
    tun_fd: jni::sys::jint,
    dns_host: jni::sys::jstring,
) {
    let Ok(mut env) = jni::JNIEnv::from_raw(env) else {
        return;
    };
    let _ = this;
    let host_jstr = jni::objects::JString::from_raw(dns_host);

    let Ok(host) = env
        .get_string(&host_jstr)
        .map(|s| s.to_string_lossy().into_owned())
    else {
        let _ = env.exception_clear();
        return;
    };
    android_tunnel::run_tunnel(tun_fd, &host);
}

/// JNI entry point to request the running tunnel loop to exit.
///
/// # Safety
///
/// Must be called with valid JNI pointers from Android runtime.
#[cfg(target_os = "android")]
#[no_mangle]
pub unsafe extern "system" fn Java_com_sicloud_dnsswitcher_DnsVpnService_nativeStopTunnel(
    _env: *mut jni::sys::JNIEnv,
    _this: jni::sys::jobject,
) {
    android_tunnel::stop_tunnel();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ip_checksum_and_packet_assembly() {
        let payload = b"\x12\x34\x01\x00\x00\x01\x00\x00\x00\x00\x00\x00"; // Mock 12-byte DNS header
        let pkt = build_ipv4_udp_packet([10, 111, 222, 2], [10, 111, 222, 1], 53, 54321, payload);

        assert_eq!(pkt.len(), 20 + 8 + payload.len());
        // Verify IPv4 header magic
        assert_eq!(pkt[0], 0x45);
        assert_eq!(pkt[9], 17); // UDP
        assert_eq!(&pkt[12..16], &[10, 111, 222, 2]);
        assert_eq!(&pkt[16..20], &[10, 111, 222, 1]);

        // RFC 791 property: Checksum over the entire 20-byte header with checksum inserted is 0
        let csum_verify = ip_checksum(&pkt[0..20]);
        assert_eq!(csum_verify, 0, "Header checksum verification failed");

        // Verify UDP header
        let src_port = u16::from_be_bytes([pkt[20], pkt[21]]);
        let dst_port = u16::from_be_bytes([pkt[22], pkt[23]]);
        let udp_len = u16::from_be_bytes([pkt[24], pkt[25]]);
        assert_eq!(src_port, 53);
        assert_eq!(dst_port, 54321);
        assert_eq!(udp_len as usize, 8 + payload.len());

        // Verify payload preservation
        assert_eq!(&pkt[28..], payload);
    }

    #[test]
    fn test_resolve_dns_host() {
        assert_eq!(
            resolve_dns_host("9.9.9.9"),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(9, 9, 9, 9)), 53))
        );
        assert_eq!(
            resolve_dns_host("dns.google"),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(8, 8, 8, 8)), 53))
        );
        assert_eq!(
            resolve_dns_host("1dot1dot1dot1.cloudflare-dns.com"),
            Some(SocketAddr::new(IpAddr::V4(Ipv4Addr::new(1, 1, 1, 1)), 53))
        );
    }

    #[test]
    fn test_dns_over_tcp_roundtrip() {
        use std::io::{Read, Write};
        use std::net::TcpListener;

        let Ok(listener) = TcpListener::bind("127.0.0.1:0") else {
            return;
        };
        let Ok(addr) = listener.local_addr() else {
            return;
        };

        // Fake DNS server: read the length-prefixed query, reply with a fixed message.
        let server = std::thread::spawn(move || {
            if let Ok((mut conn, _)) = listener.accept() {
                let mut len = [0u8; 2];
                if conn.read_exact(&mut len).is_ok() {
                    let mut q = vec![0u8; u16::from_be_bytes(len) as usize];
                    if conn.read_exact(&mut q).is_ok() {
                        let reply = b"\xaa\xbb\x01\x00";
                        let _ = conn.write_all(&(reply.len() as u16).to_be_bytes());
                        let _ = conn.write_all(reply);
                    }
                }
            }
        });

        let resp = dns_over_tcp(addr, b"\x12\x34\x01\x00\x00\x01", Duration::from_secs(2));
        assert_eq!(resp.as_deref(), Some(&b"\xaa\xbb\x01\x00"[..]));
        let _ = server.join();
    }
}
