mod control;
mod receiver;
use anyhow::{Context, Result};
use clap::Parser;
use linmic_common::{Config, Counters, State};
use serde_json::{json, Value};
use std::{
    net::IpAddr,
    os::unix::{fs::PermissionsExt, net::UnixListener},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, Mutex,
    },
    time::{Duration, Instant},
};
#[derive(Parser)]
#[command(version, about = "LinMic secure PipeWire microphone daemon")]
struct Args {
    #[arg(long)]
    no_discovery: bool,
    #[arg(long, help = "Run without PipeWire for automated network tests")]
    headless: bool,
}
#[derive(Clone)]
pub struct Session {
    pub id: u32,
    pub key: [u8; 32],
    pub ip: IpAddr,
    pub name: String,
    pub device: String,
    pub frame: usize,
    pub started: bool,
    pub heartbeat: Instant,
    pub created: Instant,
}
pub struct App {
    pub config: Config,
    pub state: State,
    pub session: Option<Session>,
    pub muted: bool,
    pub revision: u64,
    pub rtt_ms: f64,
    pub pairing: Option<(String, Instant, u8)>,
    pub devices: Value,
    pub fingerprint: String,
}
pub struct Shared {
    pub app: Mutex<App>,
    pub stats: Arc<Counters>,
    pub silent: Arc<AtomicBool>,
    pub stop: AtomicBool,
}
impl Shared {
    pub fn status(&self) -> Value {
        let a = self.app.lock().unwrap();
        let mut s = self.stats.snapshot();
        let frame = a.session.as_ref().map_or(10., |s| s.frame as f64 / 48.);
        s["state"] = json!(a.state);
        s["phone"] = json!(a.session.as_ref().map(|s| &s.name));
        s["address"] = json!(a.session.as_ref().map(|s| s.ip.to_string()));
        s["muted"] = json!(a.muted);
        s["revision"] = json!(a.revision);
        s["gain_db"] = json!(a.config.audio.gain_db);
        s["profile"] = json!(a.config.audio.default_profile);
        s["rtt_ms"] = json!(a.rtt_ms);
        s["estimated_latency_ms"] = json!(
            frame
                + a.rtt_ms / 2.
                + s["jitter_buffer_ms"].as_f64().unwrap_or(20.)
                + s["output_ring_ms"].as_f64().unwrap_or(0.)
                + s["quantum"].as_f64().unwrap_or(480.) / 48.
        );
        s["node_name"] = json!(a.config.pipewire.node_name);
        s["fingerprint"] = json!(a.fingerprint);
        s["rate"] = json!(48000);
        s["codec"] = json!("opus");
        s["encrypted"] = json!(true);
        s["health"] = json!(if a.rtt_ms < 20.
            && s["jitter_ms"].as_f64().unwrap_or(0.) < 5.
            && s["loss_percent"].as_f64().unwrap_or(0.) < 0.5
        {
            "Excellent"
        } else if a.rtt_ms < 50. && s["loss_percent"].as_f64().unwrap_or(0.) < 2. {
            "Good"
        } else {
            "Unstable"
        });
        s
    }
    pub fn disconnect(&self) {
        let mut a = self.app.lock().unwrap();
        a.session = None;
        a.state = State::Idle;
        self.silent.store(true, Ordering::Release);
        self.stats.generation.fetch_add(1, Ordering::AcqRel);
    }
}
fn main() -> Result<()> {
    let args = Args::parse();
    let mut config = Config::load()?;
    if args.no_discovery {
        config.discovery.enabled = false;
    }
    let path = linmic_common::socket_path()?;
    linmic_common::private_dir(path.parent().unwrap())?;
    if path.exists() {
        if std::os::unix::net::UnixStream::connect(&path).is_ok() {
            anyhow::bail!("linmicd is already running")
        };
        std::fs::remove_file(&path)?;
    }
    let ipc = UnixListener::bind(&path)?;
    std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))?;
    ipc.set_nonblocking(true)?;
    let identity = Arc::new(linmic_network::Identity::load()?);
    let devices_path = linmic_common::data_dir().join("devices.json");
    let devices = if devices_path.exists() {
        serde_json::from_slice(&std::fs::read(devices_path)?)?
    } else {
        json!([])
    };
    anyhow::ensure!(
        devices.as_array().is_some_and(|d| d.len() <= 32
            && d.iter()
                .all(|v| v["id"].as_str().is_some_and(|s| s.len() == 64)
                    && v["token"].as_str().is_some_and(|s| s.len() == 64)
                    && v["name"].is_string())),
        "Invalid devices.json; restore a backup or remove it to pair again"
    );
    let shared = Arc::new(Shared {
        app: Mutex::new(App {
            config: config.clone(),
            state: State::Idle,
            session: None,
            muted: false,
            revision: 0,
            rtt_ms: 0.,
            pairing: None,
            devices,
            fingerprint: identity.fingerprint.clone(),
        }),
        stats: Arc::new(Counters::default()),
        silent: Arc::new(AtomicBool::new(true)),
        stop: AtomicBool::new(false),
    });
    let (producer, consumer) = linmic_audio::RingBuffer::new(4800);
    let source = if args.headless {
        None
    } else {
        Some(linmic_pipewire::Source::new(
            &config.pipewire.node_name,
            &config.pipewire.node_description,
            consumer,
            shared.stats.clone(),
            shared.silent.clone(),
        )?)
    };
    let udp = std::net::UdpSocket::bind((config.network.bind.as_str(), config.network.audio_port))
        .context("bind audio UDP")?;
    let tcp =
        std::net::TcpListener::bind((config.network.bind.as_str(), config.network.control_port))
            .context("bind control TCP")?;
    tcp.set_nonblocking(true)?;
    let mdns = if config.discovery.enabled {
        match linmic_network::announce(
            config.network.control_port,
            &std::env::var("HOSTNAME").unwrap_or_else(|_| {
                std::fs::read_to_string("/etc/hostname")
                    .unwrap_or_else(|_| "LinMic-PC".into())
                    .trim()
                    .to_owned()
            }),
        ) {
            Ok(d) => Some(d),
            Err(e) => {
                eprintln!("mDNS unavailable: {e}");
                None
            }
        }
    } else {
        None
    };
    let s = shared.clone();
    ctrlc::set_handler(move || {
        s.stop.store(true, Ordering::Release);
        s.silent.store(true, Ordering::Release);
    })?;
    let s = shared.clone();
    let receiver = std::thread::Builder::new()
        .name("linmic-decode".into())
        .spawn(move || {
            if let Err(e) = receiver::run(udp, producer, &s) {
                eprintln!("audio worker failed: {e:#}");
                s.stop.store(true, Ordering::Release);
            }
        })?;
    let s = shared.clone();
    let control = std::thread::Builder::new()
        .name("linmic-control".into())
        .spawn(move || control::serve(tcp, identity, s))?;
    eprintln!(
        "LinMic ready · UDP {} · TLS {}",
        config.network.audio_port, config.network.control_port
    );
    let mut description = String::new();
    while !shared.stop.load(Ordering::Acquire) {
        if let Some(ref source) = source {
            if !source.poll() {
                eprintln!("PipeWire connection failed; restarting via service manager");
                break;
            }
            let name = {
                let a = shared.app.lock().unwrap();
                a.session.as_ref().map_or_else(
                    || config.pipewire.node_description.clone(),
                    |s| format!("LinMic — {}", s.name),
                )
            };
            if name != description {
                source.description(&name);
                description = name;
            }
        }
        match ipc.accept() {
            Ok((mut stream, _)) => {
                stream.set_read_timeout(Some(Duration::from_millis(300)))?;
                stream.set_write_timeout(Some(Duration::from_millis(300)))?;
                if let Ok(v) = linmic_ipc::read_json(&mut stream) {
                    let response = control::local(&shared, v)
                        .unwrap_or_else(|e| json!({"error":e.to_string()}));
                    let _ = linmic_ipc::write_json(&mut stream, &response);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(20))
            }
            Err(e) => return Err(e.into()),
        }
    }
    shared.stop.store(true, Ordering::Release);
    shared.silent.store(true, Ordering::Release);
    receiver.join().ok();
    control.join().ok();
    if let Some(d) = mdns {
        let _ = d.shutdown();
    }
    drop(source);
    std::fs::remove_file(path)?;
    Ok(())
}
