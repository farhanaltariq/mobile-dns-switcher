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
    pub permitted: bool,
}

#[cfg(target_os = "android")]
mod platform {
    use super::*;
    use jni::objects::JString;
    use jni::JNIEnv;

    fn get_app<'a>(env: &mut JNIEnv<'a>) -> Result<jni::objects::JObject<'a>, String> {
        let act_thread = env.find_class("android/app/ActivityThread").map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })?;
        let app = env
            .call_static_method(
                act_thread,
                "currentApplication",
                "()Landroid/app/Application;",
                &[],
            )
            .map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?
            .l()
            .map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;
        Ok(app)
    }

    fn get_resolver<'a>(
        env: &mut JNIEnv<'a>,
        app: &jni::objects::JObject<'a>,
    ) -> Result<jni::objects::JObject<'a>, String> {
        env.call_method(
            app,
            "getContentResolver",
            "()Landroid/content/ContentResolver;",
            &[],
        )
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })?
        .l()
        .map_err(|e| {
            let _ = env.exception_clear();
            e.to_string()
        })
    }

    fn check_permission(env: &mut JNIEnv, app: &jni::objects::JObject) -> bool {
        let Ok(perm) = env.new_string("android.permission.WRITE_SECURE_SETTINGS") else {
            let _ = env.exception_clear();
            return false;
        };
        let res = env.call_method(
            app,
            "checkSelfPermission",
            "(Ljava/lang/String;)I",
            &[(&perm).into()],
        );
        match res {
            Ok(val) => val.i().unwrap_or(-1) == 0,
            Err(_) => {
                let _ = env.exception_clear();
                false
            }
        }
    }

    pub fn get_status() -> DnsStatus {
        let vm = match ndk_context::android_context().vm() {
            vm if !vm.is_null() => unsafe { jni::JavaVM::from_raw(vm.cast()) },
            _ => {
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    permitted: false,
                }
            }
        };

        let Ok(vm) = vm else {
            return DnsStatus {
                mode: DnsMode::Unknown,
                hostname: String::new(),
                permitted: false,
            };
        };

        let Ok(mut env) = vm.attach_current_thread() else {
            return DnsStatus {
                mode: DnsMode::Unknown,
                hostname: String::new(),
                permitted: false,
            };
        };

        let app = match get_app(&mut env) {
            Ok(a) => a,
            Err(_) => {
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    permitted: false,
                }
            }
        };

        let permitted = check_permission(&mut env, &app);

        let resolver = match get_resolver(&mut env, &app) {
            Ok(r) => r,
            Err(_) => {
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    permitted,
                }
            }
        };

        let settings = match env.find_class("android/provider/Settings$Global") {
            Ok(s) => s,
            Err(_) => {
                let _ = env.exception_clear();
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    permitted,
                };
            }
        };

        let get_string = |env: &mut JNIEnv, key: &str| -> Result<String, String> {
            let jkey = env.new_string(key).map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;
            let res = env
                .call_static_method(
                    &settings,
                    "getString",
                    "(Landroid/content/ContentResolver;Ljava/lang/String;)Ljava/lang/String;",
                    &[(&resolver).into(), (&jkey).into()],
                )
                .map_err(|e| {
                    let _ = env.exception_clear();
                    e.to_string()
                })?
                .l()
                .map_err(|e| {
                    let _ = env.exception_clear();
                    e.to_string()
                })?;

            if res.is_null() {
                return Ok(String::new());
            }
            let jstr = JString::from(res);
            env.get_string(&jstr)
                .map(|s| s.to_string_lossy().into_owned())
                .map_err(|e| {
                    let _ = env.exception_clear();
                    e.to_string()
                })
        };

        let mode_str = match get_string(&mut env, "private_dns_mode") {
            Ok(m) => m,
            Err(_) => {
                return DnsStatus {
                    mode: DnsMode::Unknown,
                    hostname: String::new(),
                    permitted,
                }
            }
        };

        let host = get_string(&mut env, "private_dns_specifier")
            .or_else(|_| get_string(&mut env, "private_dns_hostname"))
            .unwrap_or_default();

        let mode = match mode_str.as_str() {
            "hostname" => DnsMode::Hostname,
            "off" => DnsMode::Off,
            _ => DnsMode::Unknown,
        };

        DnsStatus {
            mode,
            hostname: host,
            permitted,
        }
    }

    pub fn set_private_dns(hostname: Option<&str>) -> Result<(), String> {
        let vm = match ndk_context::android_context().vm() {
            vm if !vm.is_null() => unsafe { jni::JavaVM::from_raw(vm.cast()) },
            _ => return Err("Failed to obtain JavaVM".into()),
        };
        let vm = vm.map_err(|e| e.to_string())?;
        let mut env = vm.attach_current_thread().map_err(|e| e.to_string())?;

        let app = get_app(&mut env)?;
        if !check_permission(&mut env, &app) {
            return Err("WRITE_SECURE_SETTINGS not granted. Run ADB grant command.".into());
        }

        let resolver = get_resolver(&mut env, &app)?;
        let settings = env
            .find_class("android/provider/Settings$Global")
            .map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;

        let (mode, host) = match hostname {
            Some(h) => ("hostname", h),
            None => ("off", ""),
        };

        let put_string = |env: &mut JNIEnv, key: &str, val: &str| -> Result<(), String> {
            let jkey = env.new_string(key).map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;
            let jval = env.new_string(val).map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;
            env.call_static_method(
                &settings,
                "putString",
                "(Landroid/content/ContentResolver;Ljava/lang/String;Ljava/lang/String;)Z",
                &[(&resolver).into(), (&jkey).into(), (&jval).into()],
            )
            .map_err(|e| {
                let _ = env.exception_clear();
                e.to_string()
            })?;
            Ok(())
        };

        put_string(&mut env, "private_dns_mode", mode)?;
        put_string(&mut env, "private_dns_specifier", host)?;
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
            permitted: true,
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
                    permitted: true,
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
                    permitted: true,
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
