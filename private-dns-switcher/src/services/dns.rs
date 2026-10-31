#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DnsMode {
    Off,
    Hostname,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DnsStatus {
    pub mode: DnsMode,
    pub hostname: String,
    pub strict_private_dns: bool,
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;
    use jni::objects::JString;

    fn get_vm() -> Result<jni::JavaVM, String> {
        let vm = ndk_context::android_context().vm();
        if vm.is_null() {
            return Err("null VM".to_string());
        }
        unsafe { jni::JavaVM::from_raw(vm.cast()) }.map_err(|e| e.to_string())
    }

    pub fn get_status() -> DnsStatus {
        let Ok(vm) = get_vm() else {
            return DnsStatus {
                mode: DnsMode::Unknown,
                hostname: String::new(),
                strict_private_dns: false,
            };
        };

        let Ok(mut env) = vm.attach_current_thread() else {
            return DnsStatus {
                mode: DnsMode::Unknown,
                hostname: String::new(),
                strict_private_dns: false,
            };
        };

        let vpn_helper = match env.find_class("com/sicloud/dnsswitcher/VpnHelper") {
            Ok(c) => c,
            Err(_) => {
                let _ = env.exception_clear();
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    strict_private_dns: false,
                };
            }
        };

        let strict_private_dns: bool = env
            .call_static_method(&vpn_helper, "isStrictPrivateDns", "()Z", &[])
            .map(|v| v.z().unwrap_or(false))
            .unwrap_or_else(|_| {
                let _ = env.exception_clear();
                false
            });

        let is_running: bool = env
            .call_static_method(&vpn_helper, "isRunning", "()Z", &[])
            .map(|v| v.z().unwrap_or(false))
            .unwrap_or_else(|_| {
                let _ = env.exception_clear();
                false
            });

        if !is_running {
            return DnsStatus {
                mode: DnsMode::Off,
                hostname: String::new(),
                strict_private_dns,
            };
        }

        let host = env
            .call_static_method(&vpn_helper, "getActiveDns", "()Ljava/lang/String;", &[])
            .and_then(|v| v.l())
            .inspect_err(|_| {
                let _ = env.exception_clear();
            })
            .and_then(|obj| {
                if obj.is_null() {
                    Ok(String::new())
                } else {
                    let jstr = JString::from(obj);
                    env.get_string(&jstr)
                        .map(|s| s.to_string_lossy().into_owned())
                }
            })
            .unwrap_or_default();

        DnsStatus {
            mode: DnsMode::Hostname,
            hostname: host,
            strict_private_dns,
        }
    }

    pub fn set_private_dns(hostname: Option<&str>) -> Result<(), String> {
        let vm = get_vm()?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;

        let vpn_helper = env
            .find_class("com/sicloud/dnsswitcher/VpnHelper")
            .map_err(|e| {
                let _ = env.exception_clear();
                format!("find VpnHelper: {e}")
            })?;

        match hostname {
            Some(host) => {
                let jhost = env.new_string(host).map_err(|e| {
                    let _ = env.exception_clear();
                    e.to_string()
                })?;
                env.call_static_method(
                    &vpn_helper,
                    "startVpn",
                    "(Ljava/lang/String;)Z",
                    &[(&jhost).into()],
                )
                .map_err(|e| {
                    let _ = env.exception_clear();
                    format!("startVpn: {e}")
                })?;
            }
            None => {
                env.call_static_method(&vpn_helper, "stopVpn", "()Z", &[])
                    .map_err(|e| {
                        let _ = env.exception_clear();
                        format!("stopVpn: {e}")
                    })?;
            }
        }

        Ok(())
    }
}

#[cfg(not(target_os = "android"))]
mod platform {
    use super::*;
    use std::sync::Mutex;

    static MOCK_STATUS: Mutex<Option<DnsStatus>> = Mutex::new(None);

    pub fn get_status() -> DnsStatus {
        let mut lock = match MOCK_STATUS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        lock.get_or_insert_with(|| DnsStatus {
            mode: DnsMode::Hostname,
            hostname: "dns.quad9.net".into(),
            strict_private_dns: false,
        })
        .clone()
    }

    pub fn set_private_dns(hostname: Option<&str>) -> Result<(), String> {
        let mut lock = match MOCK_STATUS.lock() {
            Ok(g) => g,
            Err(p) => p.into_inner(),
        };
        match hostname {
            Some(h) => {
                *lock = Some(DnsStatus {
                    mode: DnsMode::Hostname,
                    hostname: h.to_string(),
                    strict_private_dns: false,
                });
            }
            None => {
                let current_host = lock
                    .as_ref()
                    .map(|s| s.hostname.clone())
                    .unwrap_or_default();
                *lock = Some(DnsStatus {
                    mode: DnsMode::Off,
                    hostname: current_host,
                    strict_private_dns: false,
                });
            }
        }
        Ok(())
    }
}

pub use platform::{get_status, set_private_dns};

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mock_dns_switch() {
        assert!(set_private_dns(Some("dns.google")).is_ok());
        let st = get_status();
        assert_eq!(st.mode, DnsMode::Hostname);
        assert_eq!(st.hostname, "dns.google");

        assert!(set_private_dns(None).is_ok());
        let st2 = get_status();
        assert_eq!(st2.mode, DnsMode::Off);
    }
}
