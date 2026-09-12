# Building and publishing releases

LinMic 0.2.0 targets Android 8+ (arm64-v8a, armeabi-v7a, x86_64) and Linux with PipeWire. The provided Linux binary package is built on CachyOS x86_64; older distributions may require a source build. It is not a portable AppImage or Flatpak.

1. Run the checks in README, including a short PipeWire integration test on a real user session.
2. Run `cargo build --locked --release --workspace` and `python3 scripts/generate-notices.py`.
3. Run `python3 scripts/sign-android.py`. This creates or reuses the signing identity in `~/.local/share/linmic-signing` (0700, private files 0600). Securely back up this directory outside Git. Never upload it as an artifact.
4. Verify the APK with SDK `apksigner verify --verbose --print-certs`. Release APKs must not be debuggable. A different signing key cannot update an installed APK.
5. Run `python3 scripts/package-linux.py`. Commit the reviewed source, then `python3 scripts/package-source.py` to create corresponding source and a checksummed Arch recipe.
6. Publish the signed APK, Linux tar.zst, source tar.gz, Arch recipe and SHA256SUMS alongside the matching Git tag. Use GitHub HTTPS for public downloads.

CI builds and tests source without private signing keys; its APK is a debug verification artifact, not the signed public release. To sign in a controlled build environment, provide `LINMIC_KEYSTORE` (absolute path), `LINMIC_STORE_PASSWORD`, `LINMIC_KEY_ALIAS`, and `LINMIC_KEY_PASSWORD` as environment variables to Gradle `assembleRelease`. Never put these values in source, command arguments, workflow logs or a public runner artifact. Signing and publication are deliberate maintainer actions.

For local installation, extract the Linux archive and run `./scripts/install-linux.sh` as your desktop user. The service remains active when the GUI is closed. Uninstall with `systemctl --user disable --now linmic.service`, then remove the four LinMic binaries, service and desktop/icon files installed under `~/.local`. Keep or explicitly delete `~/.config/linmic` and `~/.local/share/linmic` depending on whether you want to retain pairings.
