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

One screen with a **user-manageable list of DNS entries**. Each entry has a friendly name + hostname. The user can:

- **Add** a new DNS entry (name + hostname)
- **Edit** an existing entry
- **Delete** an entry
- **Select** an entry and **Enable** private DNS (sets `Settings.Global.PRIVATE_DNS_MODE=hostname` + `PRIVATE_DNS_HOSTNAME=<hostname>`)
- **Disable** private DNS (sets mode to `off`)

Entries persist locally as a JSON file. Three defaults are seeded on first launch (Quad9, Google, Cloudflare).

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
## File layout

Follows the fuseflow project structure (see `PLAN-structure.md` for details):

```
src/
  main.rs
  app.rs
  routes.rs
  screens/home.rs          # single screen: list + toggle
  components/status_banner.rs
  components/dns_entry_row.rs
  services/dns.rs          # JNI: read/write Settings.Global
  services/dns_store.rs    # CRUD + JSON persistence
  models/dns_entry.rs      # struct DnsEntry
  state/mod.rs             # AppState signal
```

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

### `main.rs` / `screens/home.rs` — Dioxus UI (~100 lines)

```rust
#[component]
fn HomeScreen() -> Element {
    let entries = use_signal(|| dns_store::load());
    let active_id = use_signal(|| None::<u64>);
    let dialog = use_signal(|| None::<DnsEntry>);

    rsx! {
        StatusBanner { .. }
        div { class: "dns-list",
            for entry in entries() {
                DnsEntryRow {
                    entry: entry.clone(),
                    active: active_id() == Some(entry.id),
                    on_select: move |_| { /* set active, call dns::set */ },
                    on_edit: move |_| dialog.set(Some(entry.clone())),
                    on_delete: move |_| { entries.write().retain(|e| e.id != entry.id); dns_store::save(&entries()); },
                }
            }
        }
        button { onclick: move |_| dialog.set(Some(DnsEntry::default())), "+" }
        if let Some(e) = dialog() { EditDialog { entry: e, on_save: move |e| { /* upsert */ } } }
        EnableButton { .. }
        DisableButton { .. }
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

### M2 — DNS list CRUD (1 h)
- `models/dns_entry.rs`: `DnsEntry { id, name, hostname }`
- `services/dns_store.rs`: load/save JSON to app-internal storage; seed 3 defaults
- `screens/home.rs`: render list, add/edit dialog, delete button
- Wire select → `dns::set_private_dns`

### M3 — Wire UI → JNI (30 min)
- Enable/Disable buttons call `dns::set_private_dns`
- On app start, call `dns::get_private_dns` to restore active state

### M4 — Permission guard (20 min)
- Try a harmless `Settings.Global.getString`; catch `SecurityException`
- If denied, show the `adb shell pm grant …` command as selectable text + "Retry" button

### M5 — Ship (10 min)
- App icon, label in `Dioxus.toml`
- `dx build --platform android --release` → signed APK
- README with one-time ADB setup

Total estimate: **~3–4 hours**.

## Key Cargo.toml deps

```toml
[package]
name = "private-dns-switcher"
edition = "2021"

[lib]
crate-type = ["cdylib"]

[dependencies]
dioxus = { version = "0.7", features = ["router"] }
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
jni = "0.21"

[lints.clippy]
unwrap_used = "warn"
expect_used = "warn"
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

- **Import/export DNS list** — add when users ask; it's just `serde_json::to_string` / `from_str` on the same `Vec<DnsEntry>`.
- **Quick-settings tile** — requires a `TileService` subclass via JNI; add only if toggle-from-notification is requested.
- **Root fallback path** — add a `su -c settings put …` branch in `dns.rs` if we want to support rooted devices without ADB.