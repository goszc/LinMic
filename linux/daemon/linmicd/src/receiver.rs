use super::*;
use linmic_audio::{
    jitter::{Due, JitterBuffer, Packet, Push},
    Decoder, Drift, Dsp, Producer, Sample,
};
use linmic_protocol::{Header, Replay, FEC, KEEPALIVE, MAX_DATAGRAM, MUTED};
use std::net::UdpSocket;
pub fn run(socket: UdpSocket, mut output: Producer<Sample>, s: &Shared) -> Result<()> {
    socket.set_read_timeout(Some(Duration::from_millis(2)))?;
    let config = s.app.lock().unwrap().config.clone();
    let mut jitter = JitterBuffer::new(
        config.jitter.target_ms,
        config.jitter.min_ms,
        config.jitter.max_ms,
        config.jitter.adaptive,
    );
    let mut decoder = Decoder::new()?;
    let mut dsp = Dsp::default();
    dsp.reset();
    let mut drift = Drift::default();
    let mut replay = Replay::default();
    let mut current = 0;
    let mut last_packet = None;
    let mut epoch = 0;
    let mut buffer = [0; MAX_DATAGRAM + 1];
    let mut pcm = [0i16; 960];
    let mut processed = [0f32; 960];
    let mut resampled = [0f32; 980];
    let mut misses = 0;
    while !s.stop.load(Ordering::Acquire) {
        let (session, muted, gain, gate, agc) = {
            let a = s.app.lock().unwrap();
            (
                a.session.clone(),
                a.muted,
                a.config.audio.gain_db,
                a.config.audio.noise_gate,
                a.config.audio.agc,
            )
        };
        let active = session
            .as_ref()
            .filter(|x| x.started && x.heartbeat.elapsed() < Duration::from_secs(5));
        let id = active.map_or(0, |x| x.id);
        if id != current {
            jitter.reset();
            decoder.reset();
            dsp.reset();
            drift = Drift::default();
            replay = Replay::default();
            current = id;
            last_packet = None;
            misses = 0;
            epoch = s.stats.generation.load(Ordering::Acquire);
        }
        if id == 0 {
            s.silent.store(true, Ordering::Release);
            socket.set_read_timeout(Some(Duration::from_millis(100)))?;
        } else {
            socket.set_read_timeout(Some(Duration::from_millis(2)))?;
        }
        match socket.recv_from(&mut buffer) {
            Ok((len, peer)) => {
                let result = (|| -> Result<()> {
                    let sess = active.context("no active session")?;
                    anyhow::ensure!(peer.ip() == sess.ip, "wrong source");
                    let h = Header::parse(&buffer[..len])?;
                    anyhow::ensure!(
                        h.session == sess.id && h.frame_samples as usize == sess.frame,
                        "wrong session/format"
                    );
                    let (h, data) = linmic_protocol::open(&sess.key, &mut buffer[..len])?;
                    if !replay.accept(h.sequence) {
                        s.stats.duplicate.fetch_add(1, Ordering::Relaxed);
                        return Ok(());
                    }
                    let now = Instant::now();
                    if last_packet.is_some_and(|t: Instant| {
                        now.duration_since(t) > Duration::from_millis(500)
                    }) {
                        jitter.reset();
                        decoder.reset();
                        dsp.reset();
                        drift = Drift::default();
                    }
                    last_packet = Some(now);
                    match jitter.push(Packet::new(h, data), now) {
                        Push::Accepted => {
                            s.stats.packets.fetch_add(1, Ordering::Relaxed);
                            s.stats.session_packets.fetch_add(1, Ordering::Relaxed);
                            s.stats.bytes.fetch_add(len as u64, Ordering::Relaxed);
                        }
                        Push::Duplicate => {
                            s.stats.duplicate.fetch_add(1, Ordering::Relaxed);
                        }
                        Push::Late => {
                            s.stats.late.fetch_add(1, Ordering::Relaxed);
                        }
                        Push::TooFar => {
                            s.stats.invalid.fetch_add(1, Ordering::Relaxed);
                        }
                    }
                    Ok(())
                })();
                if result.is_err() {
                    s.stats.invalid.fetch_add(1, Ordering::Relaxed);
                }
            }
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) => {}
            Err(e) => return Err(e.into()),
        }
        if id == 0 || last_packet.is_none_or(|t| t.elapsed() > Duration::from_millis(500)) {
            s.silent.store(true, Ordering::Release);
            s.stats.rms.store(0, Ordering::Relaxed);
            s.stats.peak.store(0, Ordering::Relaxed);
            for v in &s.stats.waveform {
                v.store(0, Ordering::Relaxed);
            }
            if id != 0
                && active.is_some_and(|sess| sess.created.elapsed() > Duration::from_millis(500))
            {
                let mut a = s.app.lock().unwrap();
                if a.session.as_ref().is_some_and(|x| x.id == id) {
                    a.state = State::Reconnecting;
                }
            }
            continue;
        }
        // Bound catch-up work after scheduler stalls. Excess PCM is dropped, never queued indefinitely.
        for _ in 0..8 {
            let (size, phone_muted) = match jitter.pop(Instant::now()) {
                Due::Wait => break,
                Due::Packet(p) => {
                    misses = 0;
                    let size = p.header.frame_samples as usize;
                    let silent = p.header.flags & (MUTED | KEEPALIVE) != 0;
                    if silent {
                        pcm[..size].fill(0);
                    } else {
                        match decoder.decode(Some(p.data()), &mut pcm[..size], false) {
                            Ok(n) if n == size => {}
                            _ => {
                                pcm[..size].fill(0);
                                s.stats.decode_errors.fetch_add(1, Ordering::Relaxed);
                            }
                        }
                    }
                    (size, silent)
                }
                Due::Missing(frame) => {
                    misses += 1;
                    s.stats.lost.fetch_add(1, Ordering::Relaxed);
                    s.stats.session_lost.fetch_add(1, Ordering::Relaxed);
                    let size = frame as usize;
                    if misses <= 3 {
                        let fec = jitter.peek_next().filter(|p| p.header.flags & FEC != 0);
                        let data = fec.map(|p| p.data());
                        if decoder
                            .decode(data, &mut pcm[..size], fec.is_some())
                            .is_err()
                        {
                            pcm[..size].fill(0);
                        }
                        s.stats.plc.fetch_add(1, Ordering::Relaxed);
                    } else {
                        pcm[..size].fill(0);
                    }
                    (size, false)
                }
            };
            let silent = muted || phone_muted || misses > 3;
            let (rms, peak) = dsp.process(
                &pcm[..size],
                &mut processed[..size],
                gain,
                silent,
                gate,
                agc,
            );
            let occupancy = 4800 - output.slots();
            drift.update(occupancy, 960);
            let n = drift.process(&processed[..size], &mut resampled);
            if output.slots() >= n && occupancy < 2400 {
                for &v in &resampled[..n] {
                    let _ = output.push(Sample {
                        value: v,
                        generation: epoch,
                    });
                }
            } else {
                s.stats.overruns.fetch_add(1, Ordering::Relaxed);
            }
            // A disconnect or PC mute concurrent with decoding must always win.
            let still_active = {
                let mut a = s.app.lock().unwrap();
                let active = a.session.as_ref().is_some_and(|x| x.id == id);
                if active {
                    a.state = if a.muted || phone_muted {
                        State::Muted
                    } else {
                        State::Streaming
                    };
                }
                active && !a.muted
            };
            s.silent.store(!still_active || silent, Ordering::Release);
            s.stats.rms.store(rms.to_bits(), Ordering::Relaxed);
            s.stats.peak.store(peak.to_bits(), Ordering::Relaxed);
            for (i, bucket) in processed[..size].chunks(size / 32).take(32).enumerate() {
                s.stats.waveform[i].store(
                    bucket.iter().fold(0f32, |p, x| p.max(x.abs())).to_bits(),
                    Ordering::Relaxed,
                );
            }
        }
        s.stats
            .jitter_us
            .store((jitter.jitter_ms * 1000.) as u64, Ordering::Relaxed);
        s.stats.buffer_ms.store(jitter.target_ms, Ordering::Relaxed);
        s.stats
            .ratio
            .store((drift.ratio as f32).to_bits(), Ordering::Relaxed);
    }
    Ok(())
}
