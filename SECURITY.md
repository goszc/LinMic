# Security policy

The maintained release series is 0.2.x. Do not distribute the original 0.1.0 debug APK as a production release.

Report vulnerabilities through **GitHub private vulnerability reporting** once the repository is published and its maintainer has enabled that feature. If it is unavailable, ask the maintainer for a private contact without disclosing exploit details or secrets in a public issue. Never attach audio, device tokens, private certificates, signing keys or pairing codes.

Include affected versions, reproduction steps, expected/actual behavior, platform and a minimal redacted sample. Security fixes should include regression tests and update affected release artifacts.

Implemented controls and known boundaries are described in `docs/security.md`. A dependency/advisory scan and functional tests do not constitute an independent security audit or a guarantee of absence of vulnerabilities.
