use std::net::{IpAddr, Ipv4Addr, SocketAddr, ToSocketAddrs};

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

#[cfg(target_os = "android")]
mod android_tunnel {
    use super::*;
    use std::collections::HashMap;
    use std::os::fd::AsRawFd;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::time::Instant;

    static RUNNING: AtomicBool = AtomicBool::new(false);

    pub fn stop_tunnel() {
        RUNNING.store(false, Ordering::SeqCst);
    }

    pub fn run_tunnel(
        env: &mut jni::JNIEnv,
        vpn_service: &jni::objects::JObject,
        tun_fd: i32,
        dns_host: &str,
    ) {
        RUNNING.store(true, Ordering::SeqCst);

        let Some(upstream_addr) = resolve_dns_host(dns_host) else {
            return;
        };

        let Ok(socket) = std::net::UdpSocket::bind("0.0.0.0:0") else {
            return;
        };

        // Protect the upstream socket so its traffic bypasses the VPN (prevents loops).
        let protected = env
            .call_method(
                vpn_service,
                "protect",
                "(I)Z",
                &[jni::objects::JValue::Int(socket.as_raw_fd())],
            )
            .map(|v| v.z().unwrap_or(false))
            .unwrap_or(false);
        if !protected {
            let _ = env.exception_clear();
            return;
        }

        let _ = socket.set_nonblocking(true);

        let mut in_flight: HashMap<u16, ([u8; 4], [u8; 4], u16, Instant)> = HashMap::new();

        let mut poll_fds = [
            libc::pollfd {
                fd: tun_fd,
                events: libc::POLLIN,
                revents: 0,
            },
            libc::pollfd {
                fd: socket.as_raw_fd(),
                events: libc::POLLIN,
                revents: 0,
            },
        ];

        let mut tun_buf = [0u8; 4096];
        let mut resp_buf = [0u8; 4096];

        while RUNNING.load(Ordering::Relaxed) {
            let ret = unsafe { libc::poll(poll_fds.as_mut_ptr(), 2, 250) };
            if ret < 0 {
                if std::io::Error::last_os_error().kind() == std::io::ErrorKind::Interrupted {
                    continue;
                }
                break;
            }

            if in_flight.len() > 64 {
                in_flight.retain(|_, (_, _, _, t)| t.elapsed().as_secs() < 5);
            }

            // 1. DNS query from TUN -> upstream
            if (poll_fds[0].revents & libc::POLLIN) != 0 {
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

                if n >= 28 && (tun_buf[0] >> 4) == 4 {
                    let ihl = (tun_buf[0] & 0x0F) as usize * 4;
                    if ihl >= 20 && n >= ihl + 8 && tun_buf[9] == 17 {
                        let udp_hdr = &tun_buf[ihl..ihl + 8];
                        if u16::from_be_bytes([udp_hdr[2], udp_hdr[3]]) == 53 {
                            let src_port = u16::from_be_bytes([udp_hdr[0], udp_hdr[1]]);
                            let payload_len = (u16::from_be_bytes([udp_hdr[4], udp_hdr[5]])
                                as usize)
                                .saturating_sub(8);
                            if payload_len >= 12 && ihl + 8 + payload_len <= n {
                                let src_ip: [u8; 4] =
                                    [tun_buf[12], tun_buf[13], tun_buf[14], tun_buf[15]];
                                let dst_ip: [u8; 4] =
                                    [tun_buf[16], tun_buf[17], tun_buf[18], tun_buf[19]];
                                let dns_query = &tun_buf[ihl + 8..ihl + 8 + payload_len];
                                let txid = u16::from_be_bytes([dns_query[0], dns_query[1]]);
                                in_flight.insert(txid, (src_ip, dst_ip, src_port, Instant::now()));
                                let _ = socket.send_to(dns_query, upstream_addr);
                            }
                        }
                    }
                }
            }

            // 2. DNS response from upstream -> TUN
            if (poll_fds[1].revents & libc::POLLIN) != 0 {
                while let Ok((resp_len, _)) = socket.recv_from(&mut resp_buf) {
                    if resp_len < 12 {
                        continue;
                    }
                    let txid = u16::from_be_bytes([resp_buf[0], resp_buf[1]]);
                    if let Some((client_ip, virtual_dns_ip, client_port, _)) =
                        in_flight.remove(&txid)
                    {
                        let reply = build_ipv4_udp_packet(
                            virtual_dns_ip,
                            client_ip,
                            53,
                            client_port,
                            &resp_buf[..resp_len],
                        );
                        let _ = unsafe {
                            libc::write(tun_fd, reply.as_ptr() as *const libc::c_void, reply.len())
                        };
                    }
                }
            }

            if (poll_fds[0].revents & (libc::POLLERR | libc::POLLHUP | libc::POLLNVAL)) != 0 {
                break;
            }
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
    let this_obj = jni::objects::JObject::from_raw(this);
    let host_jstr = jni::objects::JString::from_raw(dns_host);

    let Ok(host) = env
        .get_string(&host_jstr)
        .map(|s| s.to_string_lossy().into_owned())
    else {
        let _ = env.exception_clear();
        return;
    };
    android_tunnel::run_tunnel(&mut env, &this_obj, tun_fd, &host);
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
}
