# Troubleshooting

- PC not found: keep both devices on the same private network; disable wireless client isolation or use USB tethering. Enter the PC IP manually if multicast discovery is blocked. Permit TCP 39820, UDP 39821 and UDP 5353 only from the local subnet.
- Pairing fails: open a new desktop pairing window and type its six digits within 120 seconds. Five exchanges exhaust the window. If the desktop identity changed, forget the saved peer and pair again; never disable certificate verification.
- No sound: grant microphone permission, check both mute controls and select LinMic in the receiving application. Inspect `linmic status --json`, `wpctl status` and `journalctl --user -u linmic.service`.
- Low volume: raise phone gain gradually, then receiver gain if needed. Both support up to +24 dB with a limiter. A distant microphone still captures more room noise; digital gain cannot recover information absent from the source.
- Dropouts: try Stable, improve Wi-Fi signal, or use USB tethering. Avoid Bluetooth for minimum latency. Check PipeWire quantum and packet-loss metrics.
- APK installation conflict: uninstall the original 0.1.0 debug APK once. The signed 0.2.0 release uses a permanent release identity.
- Desktop does not start: install GTK4, PipeWire and Opus, or build from source for your distribution. Run `linmic-gui` in a terminal for diagnostics.
- Input name is generic: the app can only display names and routes exposed by Android. Connect the external microphone before selecting it; unplugging updates the list.
