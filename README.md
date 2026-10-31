# Private DNS Switcher

Pure Rust + Dioxus 0.7 Android mobile app to switch and benchmark DNS providers with zero root and zero ADB permissions. Ready to use immediately upon installation.

## Features

- **Ready to Use Out-of-the-Box**: Uses Android's native `VpnService` split DNS tunnel. No ADB commands, no developer options, and no root needed. (Ensure system Private DNS in Android Settings is set to "Automatic" or "Off").
- **DNS Management**: Add, edit, delete, and select custom DNS-over-TLS (DoT) providers (pre-seeded with Quad9, Google, Cloudflare).
- **Latency Testing**: Real-world TCP handshake latency test over DoT port 853 for each provider.
- **Find Best ("⚡ Test All")**: Sequentially tests all configured DNS providers and auto-selects the fastest reachable server.
- **Zero Traffic Overhead**: Only DNS queries (port 53) route through the local tunnel; all web, streaming, and app traffic flows directly at full speed.

## Prerequisites

- **Rust toolchain** + `aarch64-linux-android` target
- **Dioxus CLI**: `cargo install dioxus-cli`
- **Android SDK & NDK** (`platform-tools`, `emulator`, NDK 28+)
- **Java 17+** (`TEMURIN_17` or standard JDK 17)

## Quick Start

```bash
# Build APK, install, and launch on connected device/emulator
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
├── android/         # Minimal Kotlin NDK bridge (VpnService)
└── src/
    ├── components/  # StatusBanner, DnsEntryRow
    ├── models/      # DnsEntry data structure
    ├── screens/     # HomeScreen UI and benchmark logic
    ├── services/    # dns (JNI), dns_store (CRUD JSON), ping (TCP 853), tunnel (pure Rust UDP 53)
    ├── routes.rs    # Dioxus Router configuration
    └── main.rs      # App entry point
```

## License

[MIT](LICENSE) © 2026 Farhan Altariq
