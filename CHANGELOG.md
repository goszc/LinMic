# Changelog

## Android 0.2.1

- Keep CPU and supported background Wi-Fi locks owned by the microphone service across screen-off and reconnect intervals. Android 14+ Wi-Fi lock restrictions still apply.
- Retry temporary TLS/EOF transport failures while stopping on certificate, pairing and permission failures.
- Reload the authenticated saved identity after first pairing before reconnecting.
- Avoid closing the PC socket when an unrelated default-network callback fires; actual transport failure is detected by the existing timeout.
- Compatible with desktop 0.2.0; same APK signing identity, versionCode 3.

## 0.2.0

- Code-only desktop/Android pairing using shared SPAKE2, certificate-bound mutual confirmation and persistent identity pinning.
- Release-signed Android APK, backup exclusions and private release-key storage.
- Gain up to +24 dB, smoothed changes and envelope limiter on Android and desktop.
- Input labels based on Android device type/product name, duplicate disambiguation and hotplug updates.
- Native GTK interface, pairing-code dialog with expiration, tray and desktop-portal shortcuts.
- LAN-only control acceptance and audio bound to the authenticated TCP peer address.
- GPL-3.0-or-later licensing, build/package scripts, CI and security/contribution documentation.

## 0.1.0 (local prototype)

- Oboe capture, Opus, authenticated UDP, Linux daemon/CLI and persistent PipeWire source.
- Initial manual fingerprint pairing, basic GUI, metrics, jitter, PLC/FEC and drift correction.
