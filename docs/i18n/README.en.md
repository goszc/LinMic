# LinMic

[Português (Brasil)](../../README.md) · **English** · [Español](README.es.md) · [Русский](README.ru.md) · [简体中文](README.zh-CN.md) · [日本語](README.ja.md)

Use your Android phone as a microphone for Linux. Native apps, no account, no cloud. **0.3.0 (Android) / 0.2.0 (Linux) · Android 8+ · PipeWire Linux**, primarily CachyOS/Arch.

## Install and connect

1. Download the [latest release](https://github.com/goszc/LinMic/releases/latest).
2. Extract the Linux archive and run `./scripts/install-linux.sh` as your normal user, without sudo. Open **LinMic** from the application menu.
3. Install the Android APK. Connect the phone and PC to the same private network, or use USB tethering.
4. Click **Pair a phone** in the desktop app. On Android, select the discovered PC (or enter its IP), allow microphone access and enter the **six-digit code**. No manual fingerprint is required.
5. Select **LinMic** as the input in Discord, OBS or another app.

Codes last two minutes, are single-use and allow at most five attempts. Returning connections are authenticated automatically. The original 0.1.0 debug APK must be uninstalled once before installing the release APK, because the signing identity changed. Future releases keep the release identity.

## Audio and interface

48 kHz mono Opus; Ultra (5 ms), Balanced (10 ms), Stable (20 ms). Gain up to +24 dB with smooth changes and a -1 dBFS limiter. Android input and desktop receive gain are separate. Start at 0 dB: digital gain also raises existing noise.

Input names combine Android device type and product name, with hotplug updates. Android does not always expose individual microphone capsules. Bluetooth may add considerable latency. Displayed latency is an estimate, not an acoustic measurement.

Mute, automatic reconnect, waveform, meters, jitter/loss statistics, tray and optional portal-based global shortcuts are included. Closing the interface keeps the daemon running. Both apps support Portuguese, English, Spanish, Russian, Simplified Chinese and Japanese. Android language is in Settings → Interface; desktop has a language selector (restart the interface to apply).

## Build and check

```sh
sudo pacman -S --needed rust cmake ninja clang pkgconf pipewire opus gtk4
cargo build --locked --release --workspace
cargo test --workspace
```

Android needs JDK 17, SDK 36, NDK 28.2.13676358, CMake 3.22.1, Python and Rust targets `aarch64-linux-android`, `armv7-linux-androideabi`, `x86_64-linux-android`. Run `./gradlew assembleDebug testDebugUnitTest lintDebug` in `android/`.

Local signing: `python3 scripts/sign-android.py`. Back up `~/.local/share/linmic-signing` privately; never commit or upload it. Ports: TCP 39820, UDP 39821, mDNS UDP 5353, limited to the LAN.

[Security policy](../../SECURITY.md) · [Security design](../security.md) · [Validation](../validation.md) · [Contributing](../../CONTRIBUTING.md). Functional tests are not an independent security audit; device/distribution coverage is limited.

**GPL-3.0-or-later**. See [license](../../LICENSE) and [third-party notices](../../THIRD_PARTY_NOTICES.md). Public binary distribution must include corresponding source and notices.
