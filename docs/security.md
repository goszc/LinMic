# Security design

## Threat model

Protect microphone audio and control against passive listeners, packet injection/replay and an active attacker on an untrusted LAN. The local OS, desktop user account, Android OS/Keystore and release signing key are trust anchors. A compromised endpoint can capture microphone audio; this protocol cannot protect against that endpoint. Network denial of service remains possible.

## First pairing

The desktop user explicitly opens a 120-second pairing window. Six digits are generated with the operating system CSPRNG. At most five server exchanges can start per window. Successful use consumes the window. Closing the desktop pairing dialog revokes it.

The shared `linmic-pairing` Rust crate implements both Android and Linux participants using RustCrypto SPAKE2 0.4.0, Ed25519 group, distinct fixed application/role identities and fresh ephemeral randomness. Received group encodings must be valid, non-identity and torsion-free. The code is never transmitted and is not used as an encryption key directly.

Initial TLS is provisionally accepted by Android only for PAKE. No saved token or audio is sent to an unpinned peer. Both peers confirm the PAKE key with direction-specific HMAC-SHA256 proofs bound to the SHA-256 of the actual TLS leaf certificate. An attacker relaying SPAKE2 through a different TLS certificate cannot produce a valid proof for that certificate. Only after both confirmations does Android store the device token and certificate pin. SHA-256 is internal; users enter only the six-digit code.

This is an application-specific composition and version (`spake2-v1`), not a claim of wire compatibility with every RFC 9382 implementation. It merits independent review before broad high-assurance use. Regression tests cover wrong passwords, invalid group elements, role reflection, certificate substitution and fresh ephemerals. There is no plaintext-code fallback.

## Returning connections and audio

Saved certificate pins are checked during TLS authentication before sending a 256-bit device token. Tokens are compared without content-dependent early exit. Revocation removes the device and disconnects an active session.

Each authenticated streaming session gets a new random 256-bit audio key over TLS and a random session ID. ChaCha20-Poly1305 authenticates all 40 header bytes as associated data. Its 96-bit nonce is session ID (32-bit big endian), sequence (32-bit big endian), then four zero bytes. Senders stop before sequence exhaustion; session keys are never reused. A 64-packet replay window is updated only after tag verification. Wrong sessions, source addresses, malformed headers and datagrams over 1200 bytes are rejected.

The desktop accepts control peers only on private, link-local or loopback IP addresses. Android resolves to local/private addresses and sends audio to the actual authenticated TCP peer IP, avoiding a second hostname resolution for audio. The default desktop transport is IPv4; discovery advertises IPv4 interfaces.

## Storage, resources and lifecycle

- Desktop IPC socket: 0600 in a 0700 runtime directory. Config, private key and paired tokens use restricted files and atomic replacement.
- Android tokens: AES-GCM with a non-exportable Android Keystore key. Backup/device-transfer of app data is excluded.
- No microphone permission is requested until connecting. Foreground microphone service and persistent notification are used while streaming. Notification includes mute and disconnect.
- TCP messages are limited to 16 KiB. Control connections have timeouts and at most four worker slots. Pairing attempts have separate limits.
- Audio callbacks do not perform network I/O, blocking locks or heap allocation. Silence is generated on underflow, disconnection and mute.
- Audio is not recorded by either production app. Explicit developer receiver/integration tools can write diagnostic PCM/WAV when invoked.
- Diagnostic metrics do not contain raw audio. Logs must not include pairing codes, tokens, audio keys or full handshake JSON.

## Supply chain and publication

Source dependencies and native downloads are pinned; release workflows build from source. The APK is signed with a private release identity outside the source tree. The initial debug APK must be uninstalled once before installing the release identity. Keep signing backups; never commit them. Checksums detect accidental corruption but must be obtained through a trusted release channel to establish authenticity.

GitHub secret scanning, push protection, vulnerability alerts, automatic security fixes and private vulnerability reporting are enabled. Dependabot and CI are configured in this repository. Maintainers should require passing CI during review before merging changes. Do not publish binaries without the corresponding GPL source and third-party license notices. The local HTTP download page is a convenience on a trusted LAN, not a substitute for HTTPS-authenticated public distribution.
