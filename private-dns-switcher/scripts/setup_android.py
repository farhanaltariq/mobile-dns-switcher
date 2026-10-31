#!/usr/bin/env python3
"""Set up Android project after `dx build --platform android`.

- Injects DNS app icons from assets/icons/
- Sets app title to 'DNS Switcher' in strings.xml
- Copies Kotlin VpnService and helper classes
- Injects VpnService declarations into AndroidManifest.xml
"""

import os
import shutil
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGET_APP_DIR = os.path.join(
    BASE_DIR, "target", "dx", "private-dns-switcher", "debug", "android", "app", "app"
)
MAIN_DIR = os.path.join(TARGET_APP_DIR, "src", "main")
MANIFEST_PATH = os.path.join(MAIN_DIR, "AndroidManifest.xml")
STRINGS_PATH = os.path.join(MAIN_DIR, "res", "values", "strings.xml")

DENSITIES = {
    "mdpi": "icon_48.png",
    "hdpi": "icon_72.png",
    "xhdpi": "icon_96.png",
    "xxhdpi": "icon_144.png",
    "xxxhdpi": "icon_192.png",
}

SERVICE_SNIPPET = """        <service
            android:name="com.sicloud.dnsswitcher.DnsVpnService"
            android:permission="android.permission.BIND_VPN_SERVICE"
            android:foregroundServiceType="specialUse"
            android:exported="false">
            <intent-filter>
                <action android:name="android.net.VpnService" />
            </intent-filter>
            <property
                android:name="android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE"
                android:value="Local DNS forwarding tunnel" />
        </service>
"""

PERMISSIONS_SNIPPET = """    <uses-permission android:name="android.permission.FOREGROUND_SERVICE" />
    <uses-permission android:name="android.permission.FOREGROUND_SERVICE_SPECIAL_USE" />
"""


def main():
    if not os.path.isdir(MAIN_DIR):
        sys.exit(f"Android project not found at {MAIN_DIR}. Run `dx build --platform android` first.")

    res_dir = os.path.join(MAIN_DIR, "res")
    icons_src = os.path.join(BASE_DIR, "assets", "icons")

    # 1. Copy DNS launcher icons
    for dpi, icon_name in DENSITIES.items():
        dpi_dir = os.path.join(res_dir, f"mipmap-{dpi}")
        os.makedirs(dpi_dir, exist_ok=True)
        # Remove any dx template icons
        for old in ("ic_launcher.webp", "ic_launcher_round.webp", "ic_launcher_round.png"):
            p = os.path.join(dpi_dir, old)
            if os.path.exists(p):
                os.remove(p)
        src_icon = os.path.join(icons_src, icon_name)
        if os.path.exists(src_icon):
            shutil.copyfile(src_icon, os.path.join(dpi_dir, "ic_launcher.png"))

    # Remove adaptive-icon XML that overrides custom launcher icons
    anydpi = os.path.join(res_dir, "mipmap-anydpi-v26")
    if os.path.isdir(anydpi):
        shutil.rmtree(anydpi)

    print("ok: applied DNS launcher icons to all mipmaps")

    # 2. Update app_name in strings.xml
    if os.path.exists(STRINGS_PATH):
        with open(STRINGS_PATH, "r", encoding="utf-8") as f:
            strings = f.read()
        strings = strings.replace(
            '<string name="app_name">PrivateDnsSwitcher</string>',
            '<string name="app_name">DNS Switcher</string>'
        )
        with open(STRINGS_PATH, "w", encoding="utf-8") as f:
            f.write(strings)
        print("ok: updated app_name to 'DNS Switcher' in strings.xml")

    # 3. Copy Kotlin sources
    src_kotlin = os.path.join(BASE_DIR, "android", "src", "main", "kotlin")
    dst_kotlin = os.path.join(MAIN_DIR, "kotlin")
    if os.path.isdir(src_kotlin):
        shutil.copytree(src_kotlin, dst_kotlin, dirs_exist_ok=True)
        print("ok: copied Kotlin VPN files to target")

    # 4. Patch AndroidManifest.xml
    with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
        manifest = f.read()

    manifest = manifest.replace(
        '<uses-permission android:name="android.permission.WRITE_SECURE_SETTINGS" />',
        ""
    )

    if "android.permission.FOREGROUND_SERVICE" not in manifest:
        manifest = manifest.replace(
            '<application',
            f"{PERMISSIONS_SNIPPET}\n    <application"
        )

    if "com.sicloud.dnsswitcher.DnsVpnService" not in manifest:
        manifest = manifest.replace(
            '</application>',
            f"{SERVICE_SNIPPET}\n    </application>"
        )

    with open(MANIFEST_PATH, "w", encoding="utf-8") as f:
        f.write(manifest)
    print("ok: patched AndroidManifest.xml with DnsVpnService")


if __name__ == "__main__":
    main()
