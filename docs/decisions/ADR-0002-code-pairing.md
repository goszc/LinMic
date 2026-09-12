# ADR 0002: code-only pairing and authenticated TLS identity

Context: the user explicitly requested six-digit pairing without manual SHA-256 entry, while retaining security.

Decision: SPAKE2 from RustCrypto runs identically in a shared Rust crate on Linux and Android (static library through JNI). Directional key confirmation binds the PAKE result to the current TLS leaf certificate. Certificate pinning is automatic after successful pairing. TLS provides authenticated control transport thereafter; a fresh random audio key is distributed over it. No plaintext code fallback is supported.

Consequences: Android builds require Rust Android targets. Users only type a short-lived code. Tests must cover certificate substitution, wrong codes and malformed group messages. The PAKE/TLS composition remains subject to independent security review; tests are not a formal proof or external audit.

Alternatives: sending a short password over unverified TLS permits active interception. TOFU without confirmation does not authenticate the first connection. A custom X25519 handshake without PAKE permits offline guessing with short codes. The specification's proposed separate X25519/HKDF channel is replaced by TLS plus PAKE and fresh audio keys to avoid implementing another transport.
