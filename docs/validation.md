# Validation and known limits — 0.2.0

Validated locally on CachyOS x86_64 with PipeWire: Rust workspace tests and strict Clippy, C++ native ring/header/DSP tests, Android release unit tests and lint, and a short real TLS/PAKE → encrypted UDP/Opus → PipeWire capture test. The integration test verifies a 440 Hz tone, exact mute/disconnect silence, source persistence, reconnection, malformed packets and simulated 5% loss. Test scripts remain in the repository so results can be reproduced.

The user verified the earlier microphone path on their phone. The final APK adds pairing, gain and localization changes; final device audio quality and OEM input routing still require listening on the target handset. No long battery/thermal soak was run for the final release. Test duration was intentionally kept short at the user's request.

Latency is an estimate using network timing, queues and PipeWire quantum. It is not an acoustic round-trip measurement. Wi-Fi congestion, Bluetooth microphone profiles, Android audio preprocessing and host quantum can dominate latency. Built-in physical capsules are listed separately only if Android exposes them. Bluetooth routing differs across OEMs and OS releases.

Global shortcuts require an implemented desktop GlobalShortcuts portal. Linux archives depend on host shared libraries; source builds are recommended for distributions older than the build host. Only x86_64 Linux binaries are published, although the source recipe supports aarch64 builds. Windows/macOS desktop, direct USB protocol, Flatpak/AppImage/DEB/RPM and independent security audit are not part of this release.
