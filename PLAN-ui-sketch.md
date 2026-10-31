# UI Sketch — Private DNS Switcher

Single screen, mobile-first, dark theme by default.

## Layout (portrait, ~360×780 dp)

```
┌─────────────────────────────────────┐
│  ◀  Private DNS                     │  ← header (48dp)
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐    │
│  │  Status: ● Enabled          │    │  ← status banner (green/red)
│  │  dns.quad9.net              │    │
│  └─────────────────────────────┘    │
│                                     │
│  DNS Provider                       │  ← label
│  ┌─────────────────────────────┐    │
│  │ dns.quad9.net           ✎  │    │  ← text input
│  └─────────────────────────────┘    │
│                                     │
│  ┌─────────────────────────────┐    │
│  │  ○ dns.google               │    │  ← preset (optional, phase 2)
│  │  ○ dns.quad9.net            │    │
│  │  ○ dns.cloudflare.com       │    │
│  │  ○ Custom…                  │    │
│  └─────────────────────────────┘    │
│                                     │
│         ┌───────────────────┐       │
│         │   ENABLE / DISABLE│       │  ← primary action button
│         └───────────────────┘       │
│                                     │
├─────────────────────────────────────┤
│  ⚠ Permission not granted.          │  ← only shown if denied
│  Run:                               │
│  adb shell pm grant \               │
│    com.sicloud.dnsswitcher \        │
│    android.permission.\             │
│    WRITE_SECURE_SETTINGS            │
│                          [Retry]    │
└─────────────────────────────────────┘
```

## States

### 1. Normal (permission granted)

```
┌─────────────────────────────────────┐
│  Private DNS                        │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐    │
│  │  ● Active — dns.quad9.net   │    │  green banner
│  └─────────────────────────────┘    │
│                                     │
│  Hostname                           │
│  ┌─────────────────────────────┐    │
│  │ dns.quad9.net               │    │
│  └─────────────────────────────┘    │
│                                     │
│         ┌───────────────────┐       │
│         │     DISABLE       │       │  red-ish button
│         └───────────────────┘       │
│                                     │
└─────────────────────────────────────┘
```

### 2. Disabled state

```
┌─────────────────────────────────────┐
│  Private DNS                        │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐    │
│  │  ○ Off                      │    │  grey banner
│  └─────────────────────────────┘    │
│                                     │
│  Hostname                           │
│  ┌─────────────────────────────┐    │
│  │ dns.quad9.net               │    │  (remembers last value)
│  └─────────────────────────────┘    │
│                                     │
│         ┌───────────────────┐       │
│         │      ENABLE       │       │  green button
│         └───────────────────┘       │
│                                     │
└─────────────────────────────────────┘
```

### 3. Permission denied

```
┌─────────────────────────────────────┐
│  Private DNS                        │
├─────────────────────────────────────┤
│                                     │
│  ┌─────────────────────────────┐    │
│  │  ⚠ Permission required      │    │  yellow/red banner
│  └─────────────────────────────┘    │
│                                     │
│  This app needs WRITE_SECURE_       │
│  SETTINGS. Grant it once via ADB:   │
│                                     │
│  ┌─────────────────────────────┐    │
│  │ adb shell pm grant \        │    │  monospace, selectable
│  │   com.sicloud.dnsswitcher \ │    │
│  │   android.permission.\      │    │
│  │   WRITE_SECURE_SETTINGS     │    │
│  └─────────────────────────────┘    │
│                                     │
│         ┌───────────────────┐       │
│         │      RETRY        │       │
│         └───────────────────┘       │
│                                     │
└─────────────────────────────────────┘
```

## Component breakdown

| Component | File | Props |
|---|---|---|
| `App` (root) | `app.rs` | — |
| `HomeScreen` | `screens/home.rs` | reads `AppState` signal |
| `StatusBanner` | `components/status_banner.rs` | `mode: DnsMode`, `hostname: String`, `permitted: bool` |
| Input field | inline in `home.rs` | bound to `hostname` signal |
| Action button | inline in `home.rs` | toggles enable/disable |
| Permission block | inline in `home.rs` | shown only when `!permitted` |

## CSS sketch (`assets/styles/main.css`)

```css
:root {
  --bg: #1a1a2e;
  --surface: #16213e;
  --accent: #0f3460;
  --green: #4ecca3;
  --red: #e94560;
  --text: #eaeaea;
  --mono: "JetBrains Mono", monospace;
}

body {
  margin: 0;
  font-family: system-ui, sans-serif;
  background: var(--bg);
  color: var(--text);
}

.container {
  max-width: 400px;
  margin: 0 auto;
  padding: 24px 16px;
}

.banner {
  border-radius: 12px;
  padding: 16px;
  margin-bottom: 24px;
}
.banner--active { background: color-mix(in srgb, var(--green) 15%, transparent); border-left: 4px solid var(--green); }
.banner--off    { background: var(--surface); border-left: 4px solid #666; }
.banner--denied { background: color-mix(in srgb, var(--red) 10%, transparent); border-left: 4px solid var(--red); }

input[type="text"] {
  width: 100%;
  padding: 12px 16px;
  border-radius: 8px;
  border: 1px solid var(--accent);
  background: var(--surface);
  color: var(--text);
  font-size: 16px;
}

.btn-primary {
  width: 100%;
  padding: 14px;
  border-radius: 10px;
  border: none;
  font-size: 16px;
  font-weight: 600;
  cursor: pointer;
}
.btn-enable  { background: var(--green); color: #000; }
.btn-disable { background: var(--red);   color: #fff; }

.adb-command {
  font-family: var(--mono);
  font-size: 13px;
  background: var(--surface);
  padding: 12px;
  border-radius: 8px;
  white-space: pre-wrap;
  user-select: all;
}
```

## Interaction flow

```
App launch
  │
  ├─► dns::get_private_dns() via JNI
  │     ├─► Success → populate signals (mode, hostname)
  │     └─► SecurityException → permitted = false
  │
  ├─► Render HomeScreen
  │
  └─► User taps ENABLE/DISABLE
        │
        ├─► dns::set_private_dns(hostname) or set_private_dns(None)
        │     ├─► Success → update banner, toast "Done"
        │     └─► SecurityException → show permission block
        │
        └─► State persists (Android Settings.Global is system-wide)
```