use anyhow::Result;
use clap::Parser;
use std::{
    io::Write,
    net::UdpSocket,
    time::{Duration, Instant},
};
#[derive(Parser)]
#[command(about = "Decode authenticated datagrams to S16LE PCM (explicit diagnostic recording)")]
struct Args {
    #[arg(long, default_value = "127.0.0.1:39821")]
    bind: String,
    #[arg(long, env = "LINMIC_AUDIO_KEY")]
    key: String,
    #[arg(long)]
    session: u32,
    #[arg(long)]
    output: std::path::PathBuf,
    #[arg(long, default_value_t = 10)]
    seconds: u64,
}
fn main() -> Result<()> {
    let a = Args::parse();
    let key = linmic_network::key(&a.key)?;
    let socket = UdpSocket::bind(a.bind)?;
    socket.set_read_timeout(Some(Duration::from_millis(100)))?;
    let mut file = std::fs::File::create(a.output)?;
    let mut decoder = linmic_audio::Decoder::new()?;
    let mut replay = linmic_protocol::Replay::default();
    let mut b = [0; 1201];
    let mut pcm = [0; 960];
    let start = Instant::now();
    while start.elapsed() < Duration::from_secs(a.seconds) {
        if let Ok((n, _)) = socket.recv_from(&mut b) {
            if let Ok((h, data)) = linmic_protocol::open(&key, &mut b[..n]) {
                if h.session != a.session || !replay.accept(h.sequence) {
                    continue;
                }
                if let Ok(n) =
                    decoder.decode(Some(data), &mut pcm[..h.frame_samples as usize], false)
                {
                    for &x in &pcm[..n] {
                        file.write_all(
                            &if h.flags & linmic_protocol::MUTED != 0 {
                                0i16
                            } else {
                                x
                            }
                            .to_le_bytes(),
                        )?;
                    }
                }
            }
        }
    }
    Ok(())
}
