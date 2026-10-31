#!/usr/bin/env python3
"""Set up Android project after `dx build --platform android`.

- Injects custom launcher icons
- Copies Kotlin VpnService and helper classes
- Injects VpnService declarations into AndroidManifest.xml
"""

import os
import shutil
import subprocess
import sys

BASE_DIR = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
TARGET_APP_DIR = os.path.join(
    BASE_DIR, "target", "dx", "private-dns-switcher", "debug", "android", "app", "app"
)
MAIN_DIR = os.path.join(TARGET_APP_DIR, "src", "main")
MANIFEST_PATH = os.path.join(MAIN_DIR, "AndroidManifest.xml")

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

    # 1. Run icon generation
    gen_icon = os.path.join(BASE_DIR, "scripts", "gen_icon.py")
    subprocess.check_call([sys.executable, gen_icon, "--android"])

    # 2. Copy Kotlin sources
    src_kotlin = os.path.join(BASE_DIR, "android", "src", "main", "kotlin")
    dst_kotlin = os.path.join(MAIN_DIR, "kotlin")
    if os.path.isdir(src_kotlin):
        shutil.copytree(src_kotlin, dst_kotlin, dirs_exist_ok=True)
        print("ok: copied Kotlin VPN files to target")

    # 3. Patch AndroidManifest.xml
    with open(MANIFEST_PATH, "r", encoding="utf-8") as f:
        manifest = f.read()

    # Clean old WRITE_SECURE_SETTINGS permission
    manifest = manifest.replace(
        '<uses-permission android:name="android.permission.WRITE_SECURE_SETTINGS" />',
        ""
    )

    # Add foreground service permissions if missing
    if "android.permission.FOREGROUND_SERVICE" not in manifest:
        manifest = manifest.replace(
            '<application',
            f"{PERMISSIONS_SNIPPET}\n    <application"
        )

    # Add VpnService declaration if missing
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
