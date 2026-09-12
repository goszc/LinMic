# LinMic protocol version 1 / pairing spake2-v1

Control runs over TLS on TCP 39820. Each message is a 4-byte unsigned big-endian length followed by UTF-8 JSON; valid lengths are 1–16384. Unknown or malformed messages must not authorize streaming. Local IPC uses the same framing on a private Unix socket.

For first pairing the user opens a desktop window. A `hello` without credentials receives `pair_challenge` with a fresh SPAKE2 message and method `spake2-v1`. Android submits `pair_submit` with its message and certificate-bound proof. The server verifies it, persists a random device identity/token, consumes the code and returns `pair_success` with its confirmation proof. Android verifies that proof before storing credentials. An authenticated `hello_ack` establishes the audio session. Exact JSON fields and regression implementations live in `linux/daemon/linmicd/src/control.rs`, `crates/linmic-network` and Android `ControlClient.kt`; see `docs/security.md` for the full authentication design.

UDP datagrams are at most 1200 bytes: 40-byte header, 0–1144 bytes Opus payload, 16-byte ChaCha20-Poly1305 tag. Integers are unsigned big endian. Header fields: magic LMIC at 0 (4 bytes); version 1 at 4; codec 1 (Opus) at 5; flags at 6 (2); session at 8 (4); sequence at 12 (4); sample index at 16 (8); monotonic sender microseconds at 24 (8); frame samples at 32 (2); payload length at 34 (2); reserved zero at 36 (4).

Flags: muted=1, FEC=2, discontinuity=4, keepalive=8, encrypted=16. Unknown bits, nonzero reserved fields and inconsistent lengths are rejected. Frames are 240, 480 or 960 mono samples at 48 kHz. Empty payload requires keepalive. Production receivers require encryption.

All header bytes are AEAD associated data. Nonce is session (4), sequence (4), zero (4). New random 32-byte key per session, transmitted only through authenticated TLS. Never reuse a sequence/key pair. Stop before sequence exhaustion. Authenticate before updating the 64-packet replay window.
