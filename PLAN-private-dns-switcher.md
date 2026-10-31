# Private DNS Switcher — Android (Rust + Dioxus)

## Why Rust (not Go)

Go has no viable path to a standalone Android APK with UI. `gomobile` only produces JNI libraries that still need a Java/Kotlin host. Since the constraint is "no other language", Go is eliminated.

## Why Dioxus (not egui / raw NDK)

Dioxus gives us:
- Declarative RSX UI in pure Rust — no XML, no Kotlin, no layout files.
- First-class Android target via `dx build --platform android` (handles NDK, manifest, APK packaging).
- WebView-backed rendering — perfectly fine for a single settings screen. No OpenGL setup, no winit/egui Android backend fragility.
- Hot-reload during development (`dx serve`).

egui on Android works but requires winit + OpenGL context management and is less mature on mobile. For a one-screen utility app, Dioxus is strictly simpler.

## What the app does

One screen. A text field for a DNS hostname, a toggle (on/off), and a save button. Sets `Settings.Global.PRIVATE_DNS_HOSTNAME` and `Settings.Global.PRIVATE_DNS_MODE` (`off` | `hostname`).

## Permissions reality

| Method | Requirement |
|---|---|
| `WRITE_SECURE_SETTINGS` | Grant once via `adb shell pm grant <pkg> android.permission.WRITE_SECURE_SETTINGS`. No root needed. |
| Root (`su`) | Alternative if device is rooted. |

The app targets the ADB-grant path. On first launch, detect whether the permission is present; if not, show the exact `adb` command to copy.

## Stack

| Layer | Tool / Crate |
|---|---|
| UI framework | **Dioxus 0.6+** (RSX macros, `dx` CLI) |
| Android build | `dx build --platform android` |
| JNI bridge | `jni` crate — one call into `Settings.Global` |
| Android SDK calls | `android.provider.Settings.Global.putString` / `putInt` via JNI |
| Rendering | System WebView (fine for a single settings screen) |
| Min Android | API 28 (Android 9, where Private DNS was introduced) |

## File layout

```
private-dns-switcher/
  Dioxus.toml          # dx project config (app name, bundle id, platform)
  Cargo.toml
  src/
    main.rs            # Dioxus app entry, RSX UI (input + toggle + button)
    dns.rs             # JNI calls: read/write PRIVATE_DNS_MODE & PRIVATE_DNS_HOSTNAME
```

Two source files. That's it.

## Implementation sketch

### `dns.rs` — the actual logic (~40 lines)

```rust
use jni::JNIEnv;

pub fn set_private_dns(env: &mut JNIEnv, hostname: Option<&str>) -> Result<(), String> {
    let (mode, host) = match hostname {
        Some(h) => ("hostname", h),
        None => ("off", ""),
    };
    let settings = env.find_class("android/provider/Settings$Global")?;
    // putString(contentResolver, "private_dns_mode", mode)
    // putString(contentResolver, "private_dns_hostname", host)
    Ok(())
}

pub fn get_private_dns(env: &mut JNIEnv) -> Result<(String, String), String> {
    // getString for mode + hostname
    todo!()
}
```

### `main.rs` — Dioxus UI (~60 lines)

```rust
use dioxus::prelude::*;

fn main() {
    dioxus::launch(App);
}

#[component]
fn App() -> Element {
    let hostname = use_signal(|| "dns.quad9.net".to_string());
    let enabled = use_signal(|| true);

    rsx! {
        div { style: "padding: 24px; font-family: sans-serif;",
            h2 { "Private DNS" }
            input {
                value: "{hostname}",
                oninput: move |e| hostname.set(e.value()),
                placeholder: "e.g. dns.quad9.net",
            }
            button {
                onclick: move |_| {
                    // call dns::set_private_dns via JNI
                },
                if enabled() { "Disable" } else { "Enable" }
            }
        }
    }
}
```

## Milestones

### M0 — Skeleton APK (20 min)
- `dx new private-dns-switcher` (Dioxus CLI scaffolds the project)
- Edit `Dioxus.toml`: set bundle id, min SDK 28
- `dx build --platform android` → `adb install` → verify it launches

### M1 — JNI bridge (1 h)
- `dns.rs`: attach JNI env, call `Settings.Global.putString`
- Read-back to show current value
- Verify: `adb shell settings get global private_dns_hostname`

### M2 — Wire UI → JNI (30 min)
- Button press calls `dns::set_private_dns`
- On app start, call `dns::get_private_dns` to restore state into signals

### M3 — Permission guard (20 min)
- Try a harmless `Settings.Global.getString`; catch `SecurityException`
- If denied, show the `adb shell pm grant …` command as selectable text + "Retry" button

### M4 — Ship (10 min)
- App icon, label in `Dioxus.toml`
- `dx build --platform android --release` → signed APK
- README with one-time ADB setup

Total estimate: **~2–3 hours**.

## Key Cargo.toml deps

```toml
[package]
name = "private-dns-switcher"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dioxus = { version = "0.6", features = ["mobile"] }
jni = "0.21"
```

## Build & install

```bash
cargo install dioxus-cli          # one-time
rustup target add aarch64-linux-android
dx build --platform android --release
adb install target/dx/private-dns-switcher/release/android/app-release.apk
adb shell pm grant com.example.dnsswitcher android.permission.WRITE_SECURE_SETTINGS
```

## Risks / gotchas

1. **Dioxus Android maturity** — Dioxus 0.6 Android support is functional but newer than its web/desktop targets. If `dx build` hits NDK issues, the escape hatch is `cargo-ndk` + manual APK packaging.
2. **WebView cold-start latency** — ~200ms on older devices. Acceptable for a settings utility.
3. **No root, no ADB grant** — app is useless without one. The permission screen must make this crystal clear on first launch.

## What we skipped (and when to add)

- **DNS-over-HTTPS provider list/presets** — add when users ask; it's just a hardcoded array feeding the text field.
- **Quick-settings tile** — requires a `TileService` subclass via JNI; add only if toggle-from-notification is requested.
- **Root fallback path** — add a `su -c settings put …` branch in `dns.rs` if we want to support rooted devices without ADB.