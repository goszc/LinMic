# Validation and known limits — 0.2.0

Validated locally on CachyOS x86_64 with PipeWire: Rust workspace tests and strict Clippy, C++ native ring/header/DSP tests, Android release unit tests and lint, and a short real TLS/PAKE → encrypted UDP/Opus → PipeWire capture test. The integration test verifies a 440 Hz tone, exact mute/disconnect silence, source persistence, reconnection, malformed packets and simulated 5% loss. Test scripts remain in the repository so results can be reproduced.

The user verified the earlier microphone path on their phone. The final APK adds pairing, gain and localization changes; final device audio quality and OEM input routing still require listening on the target handset. No long battery/thermal soak was run for the final release. Test duration was intentionally kept short at the user's request.

Latency is an estimate using network timing, queues and PipeWire quantum. It is not an acoustic round-trip measurement. Wi-Fi congestion, Bluetooth microphone profiles, Android audio preprocessing and host quantum can dominate latency. Built-in physical capsules are listed separately only if Android exposes them. Bluetooth routing differs across OEMs and OS releases.

Global shortcuts require an implemented desktop GlobalShortcuts portal. Linux archives depend on host shared libraries; source builds are recommended for distributions older than the build host. Only x86_64 Linux binaries are published, although the source recipe supports aarch64 builds. Windows/macOS desktop, direct USB protocol, Flatpak/AppImage/DEB/RPM and independent security audit are not part of this release.

## Android 0.2.1 regression checks

Release lint and unit tests cover transient TLS/EOF retry, nested certificate rejection, disabled reconnection and newly saved pairing identity selection. The release APK is signed with the same identity as 0.2.0. No handset was connected over ADB during this fix: screen-off capture and OEM battery behavior must be checked on the affected phone. Legacy HIGH_PERF Wi-Fi protection is held throughout streaming/reconnecting on Android 8–13; Android 14+ replaces that mode with screen-on-only LOW_LATENCY. No Wi-Fi lock bypasses Doze or a user/OEM restriction.

## Android 0.3.0

20 Android unit tests and release lint passed. New tests distinguish quiet rooms and deliberate/system mute from frozen capture, stalled sending, stopped PC receipt and all-zero driver data; recovery limits persist across reconnects. On an Android 15 emulator paired through real SPAKE2/TLS with an isolated Linux daemon, screen-off transmission continued (795 packets over 8 seconds), the Android microphone privacy switch appeared as system silencing, and suspending/resuming audioserver triggered CAPTURE_STALLED → automatic reopen → STREAMING without user action. The emulator does not reproduce Samsung firmware or prove physical microphone quality. The affected handset still needs a screen-off check; a private diagnostic history is available if the fault recurs.

Desktop 0.3.0: live PipeWire integration verified that local monitoring links the LinMic source only after setting its independent volume; a zero-volume test confirmed the output control and child-process cleanup. Monitor unit coverage also checks explicit stop reaps the owned process.

Combined 0.3.0 validation: 21 workspace tests passed (one live PipeWire test is ignored by default and passed separately), 20 Android tests passed, release lint and desktop Clippy with warnings denied passed. APK and native desktop release builds completed.
