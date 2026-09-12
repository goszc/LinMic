use anyhow::{ensure, Context, Result};
use clap::Parser;
use linmic_network::{read_json, write_json};
use linmic_protocol::{Header, ENCRYPTED, MAX_DATAGRAM, MUTED};
use serde_json::json;
use std::{
    net::UdpSocket,
    time::{Duration, Instant},
};
#[derive(Parser)]
#[command(about = "Synthetic encrypted Opus sender; local auto-pair or remote certificate + code")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:39820")]
    host: String,
    #[arg(long, default_value_t = 440.)]
    tone: f64,
    #[arg(long, default_value_t = 10)]
    seconds: u64,
    #[arg(long, default_value = "balanced")]
    profile: String,
    #[arg(long)]
    certificate: Option<std::path::PathBuf>,
    #[arg(long)]
    code: Option<String>,
    #[arg(long, default_value_t = 0)]
    loss_every: u32,
    #[arg(long, default_value_t = 0)]
    mute_after: u64,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let (frame, bitrate, _) = linmic_common::profile(&a.profile)?;
    let local = a.host.starts_with("127.0.0.1:");
    ensure!(
        local || a.certificate.is_some(),
        "remote tests require --certificate and --code"
    );
    let cert = std::fs::read(
        a.certificate
            .unwrap_or_else(|| linmic_common::data_dir().join("server.der")),
    )?;
    let code = if let Some(code) = a.code {
        code
    } else {
        ensure!(local, "--code required");
        linmic_ipc::request(json!({"type":"pair"}))?["code"]
            .as_str()
            .context("pair code")?
            .to_string()
    };
    let mut control = linmic_network::connect(&a.host, &cert)?;
    write_json(
        &mut control,
        &json!({"type":"hello","protocol":1,"client":{"name":"Synthetic 440 Hz"},"profile":a.profile}),
    )?;
    let mut response = read_json(&mut control)?;
    if response["type"] == "pair_challenge" {
        linmic_network::pair_client(&mut control, &response, &code, &cert)?;
        response = read_json(&mut control)?;
    }
    ensure!(response["type"] == "hello_ack", "hello failed: {response}");
    let id = response["session_id"].as_u64().context("session")? as u32;
    let key = linmic_network::key(response["audio_key"].as_str().context("key")?)?;
    let peer = control.sock.peer_addr()?;
    let udp = UdpSocket::bind(if peer.is_ipv4() {
        "0.0.0.0:0"
    } else {
        "[::]:0"
    })?;
    udp.connect((
        peer.ip(),
        response["udp_port"].as_u64().context("port")? as u16,
    ))?;
    write_json(&mut control, &json!({"type":"start_stream"}))?;
    let _ = read_json(&mut control)?;
    let mut encoder = linmic_audio::Encoder::new(bitrate, frame == 240)?;
    let mut pcm = vec![0i16; frame];
    let mut packet = [0; MAX_DATAGRAM];
    let mut encoded = [0; linmic_protocol::MAX_PAYLOAD];
    let start = Instant::now();
    let mut due = start;
    let mut ping = start;
    let mut seq = 0u32;
    let mut muted = false;
    while start.elapsed() < Duration::from_secs(a.seconds) {
        if seq == u32::MAX {
            break;
        }
        if ping.elapsed() >= Duration::from_millis(100) {
            let t = Instant::now();
            write_json(
                &mut control,
                &json!({"type":"ping","t0":start.elapsed().as_micros()as u64}),
            )?;
            let stats = read_json(&mut control)?;
            muted = stats["muted"].as_bool().unwrap_or(false);
            ping = t;
        }
        let mute = muted || (a.mute_after > 0 && start.elapsed().as_secs() >= a.mute_after);
        for (i, x) in pcm.iter_mut().enumerate() {
            *x = if mute {
                0
            } else {
                (((seq as u64 * frame as u64 + i as u64) as f64 * a.tone * std::f64::consts::TAU
                    / 48000.)
                    .sin()
                    * 8192.) as i16
            };
        }
        let len = encoder.encode(&pcm, &mut encoded)?;
        let h = Header {
            flags: ENCRYPTED | if mute { MUTED } else { 0 },
            session: id,
            sequence: seq,
            sample_index: seq as u64 * frame as u64,
            sender_time_us: start.elapsed().as_micros() as u64,
            frame_samples: frame as u16,
            payload_len: len as u16,
        };
        let len = linmic_protocol::seal(&key, &h, &encoded[..len], &mut packet)?;
        if a.loss_every == 0 || seq % a.loss_every != a.loss_every - 1 {
            udp.send(&packet[..len])?;
        }
        seq = seq.wrapping_add(1);
        due += Duration::from_micros(frame as u64 * 1_000_000 / 48000);
        std::thread::sleep(due.saturating_duration_since(Instant::now()));
    }
    write_json(&mut control, &json!({"type":"disconnect"}))?;
    let _ = read_json(&mut control);
    println!("Sent {seq} encrypted Opus frames");
    Ok(())
}
