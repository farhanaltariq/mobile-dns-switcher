# Folder Structure — Private DNS Switcher

## Why follow fuseflow's format

Consistency across the workspace. `rust-mobile/fuseflow` already establishes the Dioxus project conventions for this team:

- Same `Dioxus.toml` layout (application, android, bundle sections)
- Same `src/` subdirectory pattern (screens, components, services, models, state, utils)
- Same `assets/` split (icons, styles)
- Same tooling files (`clippy.toml`, `rustfmt.toml`)

A developer who knows fuseflow can open this project and immediately know where everything is. No new conventions to learn. The structure scales: if we later add a presets screen or a tile service, the directories already exist as the natural home.

## Structure (scaled to actual needs)

```
private-dns-switcher/
├── .gitignore
├── Cargo.toml
├── Cargo.lock
├── Dioxus.toml              # app name, bundle id, android config
├── clippy.toml              # lint config (same rules as fuseflow)
├── rustfmt.toml             # formatting (same rules as fuseflow)
│
├── assets/
│   ├── icons/
│   │   └── icon.png         # app launcher icon (512×512)
│   └── styles/
│       └── main.css         # single stylesheet for the WebView UI
│
├── src/
│   ├── main.rs              # entry: dioxus::launch(App)
│   ├── app.rs               # root component, router outlet
│   ├── routes.rs            # route enum (Home only; extensible later)
│   │
│   ├── screens/
│   │   ├── mod.rs
│   │   └── home.rs          # THE screen: DNS list + enable/disable + add/edit dialog
│   │
│   ├── components/
│   │   ├── mod.rs
│   │   ├── status_banner.rs # enabled/disabled/permission-denied indicator
│   │   └── dns_entry_row.rs # one row: name, hostname, radio dot, edit/delete buttons
│   │
│   ├── services/
│   │   ├── mod.rs
│   │   ├── dns.rs           # JNI: read/write Settings.Global (PRIVATE_DNS_MODE/HOSTNAME)
│   │   └── dns_store.rs     # CRUD for user's DNS list (local JSON file persistence)
│   │
│   ├── models/
│   │   ├── mod.rs
│   │   └── dns_entry.rs     # struct DnsEntry { id, name, hostname }
│   │
│   ├── state/
│   │   └── mod.rs           # AppState signal: entries, active_id, permitted
│   │
│   └── utils/
│       └── mod.rs           # empty for now; hostname validation if needed
│
└── scripts/
    └── gen_icon.py          # copy from fuseflow
```

## Mapping to fuseflow

| fuseflow dir | Purpose | Our use |
|---|---|---|
| `screens/` | One file per routable screen | `home.rs` (only screen) |
| `components/` | Reusable UI widgets | `status_banner.rs`, `dns_entry_row.rs` |
| `services/` | Business logic / platform calls | `dns.rs` (JNI), `dns_store.rs` (CRUD + persistence) |
| `models/` | Data structs | `dns_entry.rs` |
| `state/` | Global app state | `mod.rs` (entries + active + permission) |
| `utils/` | Shared helpers | Empty initially |
| `assets/styles/` | CSS for WebView | `main.css` |
| `assets/icons/` | App icon | `icon.png` |

## What we DON'T create

- No `i18n.rs` — single language, no locale switching needed.
- No `routes.rs` with multiple variants — one route. File exists but has one variant.
- No empty `utils/` files with placeholder functions — `mod.rs` only.
- No separate `edit_dialog.rs` component — the dialog is 15 lines of RSX inline in `home.rs`. Extract only if it grows.

## Config files (mirror fuseflow)

**`Dioxus.toml`**
```toml
[application]
name = "private-dns-switcher"
default_platform = "android"
out_dir = "dist"
asset_dir = "assets"

[android]
package = "com.sicloud.dnsswitcher"
label = "DNS Switcher"
min_sdk_version = 28
target_sdk_version = 35

[bundle]
identifier = "com.sicloud.dnsswitcher"
publisher = "SICloud"
icon = ["assets/icons/icon.png"]
```

**`clippy.toml`** — copy from fuseflow.
**`rustfmt.toml`** — copy from fuseflow.