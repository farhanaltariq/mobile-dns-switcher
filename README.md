# Private DNS Switcher

Pure Rust Android mobile app built with [Dioxus 0.7](https://dioxuslabs.com/) to manage and switch Android Private DNS (`Settings.Global: private_dns_mode`, `private_dns_specifier`) with built-in latency benchmarking.

## Features

- **DNS Management**: Add, edit, delete, and select custom DNS-over-TLS (DoT) providers (pre-seeded with Quad9, Google, Cloudflare).
- **One-Touch Toggle**: Turn Private DNS on/off instantly via Android secure settings.
- **Latency Testing**: Real-world TCP handshake latency test over DoT port 853 for each provider.
- **Find Best ("⚡ Test All")**: Sequentially tests all configured DNS providers and auto-selects the fastest reachable server.
- **Pure Rust**: 100% Rust logic via JNI calls (`android.provider.Settings.Global`) with no Kotlin/Java boilerplate.

## Prerequisites

- **Rust toolchain** + `aarch64-linux-android` target
- **Dioxus CLI**: `cargo install dioxus-cli`
- **Android SDK & NDK** (`platform-tools`, `emulator`, NDK 28+)
- **Java 17+** (`TEMURIN_17` or standard JDK 17)

## One-Time Permission Setup

Android restricts writing secure global settings to system apps or ADB-granted apps:

```bash
adb shell pm grant com.sicloud.dnsswitcher android.permission.WRITE_SECURE_SETTINGS
# or run:
make grant-permission
```

## Quick Start

```bash
# Build APK, install, grant permission, and launch on connected device/emulator
make run

# Build APK only
make apk

# Run desktop preview (mock DNS mode)
make desktop

# Run unit tests
cargo test --manifest-path private-dns-switcher/Cargo.toml
```

## Project Structure

Follows the `fuseflow` feature-oriented pattern:

```text
private-dns-switcher/
├── assets/          # CSS themes and launcher icons
├── scripts/         # Icon injection and asset utilities
└── src/
    ├── components/  # StatusBanner, DnsEntryRow
    ├── models/      # DnsEntry data structure
    ├── screens/     # HomeScreen UI and benchmark logic
    ├── services/    # dns (JNI), dns_store (CRUD JSON), ping (TCP 853)
    ├── routes.rs    # Dioxus Router configuration
    └── main.rs      # App entry point
```

## License

[MIT](LICENSE) © 2026 Farhan Altariq
