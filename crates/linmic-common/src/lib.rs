use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    fs,
    os::unix::fs::{OpenOptionsExt, PermissionsExt},
    path::PathBuf,
    sync::atomic::{AtomicBool, AtomicU32, AtomicU64, Ordering},
};

pub const RATE: usize = 48_000;
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum State {
    Idle,
    Discovering,
    Pairing,
    Connecting,
    Connected,
    Streaming,
    Muted,
    Reconnecting,
    Stopping,
    Error,
}
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Config {
    pub network: Network,
    pub audio: Audio,
    pub jitter: Jitter,
    pub pipewire: Pipewire,
    pub discovery: Discovery,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Network {
    pub bind: String,
    pub control_port: u16,
    pub audio_port: u16,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Audio {
    pub gain_db: f32,
    pub default_profile: String,
    pub noise_gate: bool,
    pub agc: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Jitter {
    pub target_ms: u32,
    pub min_ms: u32,
    pub max_ms: u32,
    pub adaptive: bool,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Pipewire {
    pub node_name: String,
    pub node_description: String,
}
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct Discovery {
    pub enabled: bool,
}
impl Default for Network {
    fn default() -> Self {
        Self {
            bind: "0.0.0.0".into(),
            control_port: 39820,
            audio_port: 39821,
        }
    }
}
impl Default for Audio {
    fn default() -> Self {
        Self {
            gain_db: 0.,
            default_profile: "balanced".into(),
            noise_gate: false,
            agc: false,
        }
    }
}
impl Default for Jitter {
    fn default() -> Self {
        Self {
            target_ms: 20,
            min_ms: 10,
            max_ms: 80,
            adaptive: true,
        }
    }
}
impl Default for Pipewire {
    fn default() -> Self {
        Self {
            node_name: "linmic_input".into(),
            node_description: "LinMic".into(),
        }
    }
}
impl Default for Discovery {
    fn default() -> Self {
        Self { enabled: true }
    }
}

pub fn home() -> PathBuf {
    std::env::var_os("HOME")
        .map(PathBuf::from)
        .expect("HOME is required")
}
pub fn config_path() -> PathBuf {
    std::env::var_os("XDG_CONFIG_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".config"))
        .join("linmic/config.toml")
}
pub fn data_dir() -> PathBuf {
    std::env::var_os("XDG_DATA_HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| home().join(".local/share"))
        .join("linmic")
}
pub fn socket_path() -> Result<PathBuf> {
    Ok(PathBuf::from(
        std::env::var_os("XDG_RUNTIME_DIR")
            .context("XDG_RUNTIME_DIR is required; run as your desktop user")?,
    )
    .join("linmic/linmic.sock"))
}
pub fn private_dir(p: &std::path::Path) -> Result<()> {
    fs::create_dir_all(p)?;
    fs::set_permissions(p, fs::Permissions::from_mode(0o700))?;
    Ok(())
}
pub fn private_write(p: &std::path::Path, data: &[u8]) -> Result<()> {
    use std::io::Write;
    private_dir(p.parent().context("missing parent")?)?;
    let tmp = p.with_extension("tmp");
    // O_NOFOLLOW prevents following a pre-existing symlink at the temporary path.
    let mut f = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&tmp)?;
    let result = (|| {
        f.write_all(data)?;
        f.sync_all()?;
        fs::rename(&tmp, p)?;
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(tmp);
    }
    result
}
impl Config {
    pub fn load() -> Result<Self> {
        let p = config_path();
        if !p.exists() {
            return Ok(Self::default());
        }
        let c: Self = toml::from_str(&fs::read_to_string(p)?)?;
        c.validate()?;
        Ok(c)
    }
    pub fn validate(&self) -> Result<()> {
        if !self.audio.gain_db.is_finite() || !(-60.0..=24.0).contains(&self.audio.gain_db) {
            bail!("gain must be -60..24 dB")
        };
        if !(5..=80).contains(&self.jitter.min_ms)
            || self.jitter.min_ms > self.jitter.target_ms
            || self.jitter.target_ms > self.jitter.max_ms
            || self.jitter.max_ms > 100
        {
            bail!("invalid jitter limits")
        };
        if self.network.control_port == 0 || self.network.audio_port == 0 {
            bail!("ports cannot be zero")
        };
        profile(&self.audio.default_profile)?;
        Ok(())
    }
    pub fn save(&self) -> Result<()> {
        self.validate()?;
        private_write(&config_path(), toml::to_string_pretty(self)?.as_bytes())
    }
}
pub fn profile(name: &str) -> Result<(usize, u32, u32)> {
    match name {
        "ultra" => Ok((240, 64000, 10)),
        "balanced" => Ok((480, 48000, 20)),
        "stable" => Ok((960, 48000, 50)),
        _ => bail!("profile must be ultra, balanced or stable"),
    }
}
#[derive(Default)]
pub struct Counters {
    pub packets: AtomicU64,
    pub bytes: AtomicU64,
    pub invalid: AtomicU64,
    pub duplicate: AtomicU64,
    pub lost: AtomicU64,
    pub late: AtomicU64,
    pub plc: AtomicU64,
    pub decode_errors: AtomicU64,
    pub underruns: AtomicU64,
    pub overruns: AtomicU64,
    pub rms: AtomicU32,
    pub peak: AtomicU32,
    pub jitter_us: AtomicU64,
    pub buffer_ms: AtomicU32,
    pub output_samples: AtomicU32,
    pub ratio: AtomicU32,
    pub node_id: AtomicU32,
    pub quantum: AtomicU32,
    pub pipewire_connected: AtomicBool,
    pub session_packets: AtomicU64,
    pub session_lost: AtomicU64,
    pub waveform: [AtomicU32; 32],
    pub generation: AtomicU64,
}
impl Counters {
    pub fn snapshot(&self) -> serde_json::Value {
        use serde_json::json;
        let get = |a: &AtomicU64| a.load(Ordering::Relaxed);
        let packets = get(&self.packets);
        let lost = get(&self.lost);
        let sp = get(&self.session_packets);
        let sl = get(&self.session_lost);
        json!({"packets_received":packets,"session_packets":get(&self.session_packets),"bytes_received":get(&self.bytes),"invalid_packets":get(&self.invalid),"duplicate_packets":get(&self.duplicate),"packets_lost":lost,"late_packets":get(&self.late),"plc_frames":get(&self.plc),"decoder_errors":get(&self.decode_errors),"pipewire_underruns":get(&self.underruns),"output_overruns":get(&self.overruns),"rms_db":db(f32::from_bits(self.rms.load(Ordering::Relaxed))),"peak_db":db(f32::from_bits(self.peak.load(Ordering::Relaxed))),"jitter_ms":get(&self.jitter_us) as f64/1000.,"jitter_buffer_ms":self.buffer_ms.load(Ordering::Relaxed),"output_ring_ms":self.output_samples.load(Ordering::Relaxed) as f64/48.,"loss_percent":if sp+sl==0 {0.} else {sl as f64*100./(sp+sl) as f64},"lifetime_loss_percent":if packets+lost==0 {0.} else {lost as f64*100./(packets+lost) as f64},"resample_ratio":f32::from_bits(self.ratio.load(Ordering::Relaxed)),"pipewire_connected":self.pipewire_connected.load(Ordering::Relaxed),"node_id":self.node_id.load(Ordering::Relaxed),"quantum":self.quantum.load(Ordering::Relaxed),"waveform":self.waveform.iter().map(|a|f32::from_bits(a.load(Ordering::Relaxed))).collect::<Vec<_>>()})
    }
}
pub fn db(v: f32) -> f32 {
    20. * v.max(0.000001).log10()
}
pub fn monotonic_us() -> u64 {
    static ORIGIN: std::sync::OnceLock<std::time::Instant> = std::sync::OnceLock::new();
    ORIGIN
        .get_or_init(std::time::Instant::now)
        .elapsed()
        .as_micros() as u64
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn validation() {
        let mut c = Config::default();
        assert!(c.validate().is_ok());
        c.audio.gain_db = f32::NAN;
        assert!(c.validate().is_err());
    }
    #[test]
    fn profiles() {
        assert_eq!(profile("balanced").unwrap().0, 480);
        assert!(profile("unknown").is_err());
    }
}
