# Android SDK & Tools
export ANDROID_HOME     ?= $(HOME)/Library/Android/sdk
export ANDROID_NDK_HOME ?= $(ANDROID_HOME)/ndk/28.0.13004108
export JAVA_HOME        ?= /Library/Java/JavaVirtualMachines/temurin-17.jdk/Contents/Home
export PATH             := $(PATH):$(ANDROID_HOME)/platform-tools:$(ANDROID_HOME)/emulator

AVD_NAME := Pixel_3a_API_34_extension_level_7_arm64-v8a
PKG      := com.sicloud.dnsswitcher
APP_NAME := private-dns-switcher
APK      := target/dx/$(APP_NAME)/debug/android/app/app/build/outputs/apk/debug/app-debug.apk

.DEFAULT_GOAL := help

.PHONY: help
help: ## Show this help
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | awk 'BEGIN {FS = ":.*?## "}; {printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2}'

.PHONY: doctor
doctor: ## Check toolchain and environment
	@dx doctor

.PHONY: check
check: ## Run format check, linter, and tests
	cargo fmt --check && cargo clippy -- -D warnings && cargo test

.PHONY: desktop
desktop: ## Build and run on desktop (fastest feedback loop)
	dx serve --platform desktop

.PHONY: emu
emu: emu-kill ## Kill running emulator and boot a fresh one (GUI)
	@sleep 1
	emulator -avd $(AVD_NAME)

.PHONY: emu-headless
emu-headless: ## Boot emulator in background without GUI
	emulator -avd $(AVD_NAME) -no-window -no-audio -no-boot-anim -gpu swiftshader_indirect &
	adb wait-for-device
	@echo "Emulator ready."

.PHONY: emu-wait
emu-wait: ## Wait for emulator to fully finish booting
	adb wait-for-device
	@while [ -z "$$(adb shell getprop sys.boot_completed 2>/dev/null | tr -d '\r')" ]; do sleep 1; done
	@echo "Device booted."

.PHONY: emu-kill
emu-kill: ## Kill running emulator
	adb emu kill 2>/dev/null || true

.PHONY: build-android
build-android: ## Build Android project via Dioxus CLI
	rm -f target/dx/$(APP_NAME)/debug/android/app/app/src/main/res/mipmap-*/ic_launcher.*
	dx build --platform android

.PHONY: patch-android
patch-android: ## Patch generated Android project with VPN service, launcher icons, and title
	@python3 -c "import os, glob, shutil; \
app = 'target/dx/$(APP_NAME)/debug/android/app/app/src/main'; res = os.path.join(app, 'res'); \
shutil.rmtree(os.path.join(res, 'mipmap-anydpi-v26'), ignore_errors=True); \
[os.remove(f) for f in glob.glob(os.path.join(res, 'mipmap-*', 'ic_launcher.webp'))]; \
[shutil.copyfile(f'assets/icons/icon_{sz}.png', os.path.join(res, f'mipmap-{dpi}', 'ic_launcher.png')) for dpi, sz in [('mdpi','48'),('hdpi','72'),('xhdpi','96'),('xxhdpi','144'),('xxxhdpi','192')] if os.path.exists(f'assets/icons/icon_{sz}.png')]; \
shutil.copytree('android/src/main/kotlin', os.path.join(app, 'kotlin'), dirs_exist_ok=True); \
sp = os.path.join(res, 'values', 'strings.xml'); (lambda c: open(sp,'w').write(c.replace('PrivateDnsSwitcher','DNS Switcher')))(open(sp).read()) if os.path.exists(sp) else None; \
mp = os.path.join(app, 'AndroidManifest.xml'); (lambda c: open(mp,'w').write(c.replace('</application>', '  <service android:name=\"com.sicloud.dnsswitcher.DnsVpnService\" android:permission=\"android.permission.BIND_VPN_SERVICE\" android:foregroundServiceType=\"specialUse\" android:exported=\"false\"><intent-filter><action android:name=\"android.net.VpnService\" /></intent-filter><property android:name=\"android.app.PROPERTY_SPECIAL_USE_FGS_SUBTYPE\" android:value=\"Local DNS forwarding tunnel\" /></service>\n    </application>').replace('<uses-permission android:name=\"android.permission.WRITE_SECURE_SETTINGS\" />', '')))(open(mp).read()) if os.path.exists(mp) and 'DnsVpnService' not in open(mp).read() else None"

.PHONY: apk
apk: build-android patch-android ## Generate Android APK file with app launcher icons & VPN service
	cd target/dx/$(APP_NAME)/debug/android/app && ./gradlew --quiet assembleDebug
	@echo "APK ready: $(APK)"

.PHONY: install
install: ## Install built APK onto connected device/emulator
	adb install -r $(APK)

.PHONY: launch
launch: ## Launch the app on connected device/emulator
	adb shell am start -n $(PKG)/dev.dioxus.main.MainActivity

.PHONY: run
run: apk install launch ## Build, install, and launch on device (no ADB permissions needed!)

.PHONY: logs
logs: ## Stream app logs from device
	adb logcat -s $(PKG) RustStdoutStderr

.PHONY: clean
clean: ## Clean build artifacts
	cargo clean
