# Contributing

Contributions are under GPL-3.0-or-later. Keep the daemon independent of the GUI and maintain Android → Opus → authenticated UDP → PipeWire as the primary path.

Before proposing a change, run the Rust formatting/lint/tests, native CTest and Android lint/unit tests. Changes to protocol, pairing or DSP need meaningful regression tests. Wire changes must be versioned and documented. Do not put network calls, allocation or blocking locks into audio callbacks.

Use synthetic audio for reports and tests. Redact IPs/names when appropriate; never submit microphone recordings, tokens, codes, signing material or local configuration. Describe tested devices/OS versions and any untested assumptions.

Do not update cryptographic dependencies or protocol composition without examining upstream advisories and rerunning negative security tests. Public releases require source archives and licenses alongside binaries.
