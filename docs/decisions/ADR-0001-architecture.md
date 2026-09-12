# ADR 0001: native Linux/Android architecture

Context: the supplied specification prioritizes correctness, stability and low overhead.

Decision: Rust daemon and CLI; optional Rust/GTK4 interface; Kotlin/XML Android with C++ Oboe and Opus. A small C bridge encapsulates PipeWire's stable C ABI instead of relying on Rust bindings. It negotiates mono F32LE at 48 kHz. The producer passes generation-tagged samples through rtrb; the callback discards stale generations and writes zeros when muted/disconnected.

Consequences: a C compiler and PipeWire headers are build dependencies. Audio stays off UI/control threads. No Electron, kernel driver or root daemon is needed. Network receive and decode share one bounded worker; no per-packet thread or jitter-slot allocation.

Alternatives: Rust PipeWire bindings would also work; the narrow C bridge made API and realtime behavior explicit. S16LE output was optional; F32LE avoids extra post-DSP quantization.
