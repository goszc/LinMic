# Architecture

Android uses a foreground microphone service. An Oboe callback copies mono 48 kHz PCM into a bounded SPSC ring. A worker applies smoothed gain and limiting, encodes Opus and sends authenticated UDP. Control, discovery and UI run outside the realtime callback.

The Linux daemon owns TLS control, discovery, bounded UDP ingress, a preallocated jitter buffer, Opus decoder, drift resampling and receiver DSP. A bounded ring bridges into a C PipeWire callback exposed by the Rust owner. The callback only consumes samples or writes silence. Disconnect invalidates old generations immediately while preserving the virtual source.

The GTK4 interface, CLI and optional tray use a private Unix socket. The GUI can close without stopping capture. Android and Linux share the Rust SPAKE2 implementation through a small JNI/C ABI on Android. See the decision records and security design for the rationale.
