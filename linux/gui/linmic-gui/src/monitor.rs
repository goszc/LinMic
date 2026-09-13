//! User-requested local monitoring. The child belongs to this GUI, never the daemon.
use anyhow::{bail, Context, Result};
use serde_json::Value;
use std::os::unix::process::CommandExt;
use std::process::{Child, Command, Stdio};

#[derive(Default)]
pub struct Monitor {
    child: Option<Child>,
    pub volume: f64,
    node: Option<u64>,
    source: String,
    linked: bool,
    started: Option<std::time::Instant>,
}
impl Monitor {
    pub fn volume_ready(&self) -> bool {
        self.node.is_some() && self.linked
    }
    pub fn stop(&mut self) {
        if let Some(mut child) = self.child.take() {
            let _ = child.kill();
            let _ = child.wait();
        }
        self.node = None;
        self.linked = false;
        self.started = None;
    }
    pub fn running(&mut self) -> bool {
        match self.child.as_mut().map(Child::try_wait) {
            Some(Ok(None)) => true,
            _ => {
                self.stop();
                false
            }
        }
    }
    pub fn start(&mut self, source: &str) -> Result<()> {
        self.stop();
        if source.is_empty() {
            bail!("No LinMic source available");
        }
        self.source = source.to_owned();
        self.started = Some(std::time::Instant::now());
        let mut command = Command::new("pw-loopback");
        let parent = std::process::id() as libc::pid_t;
        // Stop monitoring if the GUI exits, including termination without Rust cleanup.
        unsafe {
            command.pre_exec(move || {
                if libc::prctl(libc::PR_SET_PDEATHSIG, libc::SIGTERM) == -1 {
                    return Err(std::io::Error::last_os_error());
                }
                if libc::getppid() != parent {
                    return Err(std::io::Error::other("GUI exited"));
                }
                Ok(())
            });
        }
        self.child = Some(command
            .args(["--name", "linmic_local_monitor", "--channels", "1", "--channel-map", "MONO",
                "--capture", source, "--capture-props", "node.name=linmic_monitor_capture node.dont-fallback=true node.passive=true node.autoconnect=false",
                "--playback-props", "node.name=linmic_monitor_playback media.role=Music state.restore-props=false node.dont-fallback=true"])
            .stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null())
            .spawn().context("pw-loopback could not start")?);
        Ok(())
    }
    pub fn apply_volume(&mut self) -> Result<()> {
        if self.child.is_none() {
            return Ok(());
        }
        let graph: Vec<Value> = if self.node.is_none() || !self.linked {
            let output = Command::new("pw-dump")
                .output()
                .context("pw-dump unavailable")?;
            if !output.status.success() {
                bail!("PipeWire unavailable");
            }
            serde_json::from_slice(&output.stdout)?
        } else {
            Vec::new()
        };
        let pid = self.child.as_ref().unwrap().id() as u64;
        let client = graph
            .iter()
            .find(|v| {
                let p = &v["info"]["props"]["application.process.id"];
                v["type"] == "PipeWire:Interface:Client"
                    && (p.as_u64() == Some(pid)
                        || p.as_str().and_then(|s| s.parse().ok()) == Some(pid))
            })
            .and_then(|v| v["id"].as_u64());
        if self.node.is_none() {
            self.node = graph
                .iter()
                .find(|v| {
                    v["info"]["props"]["node.name"] == "linmic_monitor_playback"
                        && client.is_some()
                        && v["info"]["props"]["client.id"].as_u64() == client
                })
                .and_then(|v| v["id"].as_u64());
        }
        if let Some(id) = self.node {
            let status = Command::new("wpctl")
                .args([
                    "set-volume",
                    &id.to_string(),
                    &format!("{:.2}", self.volume.clamp(0., 1.)),
                ])
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .status()?;
            if !status.success() {
                bail!("Monitor volume unavailable");
            }
        }
        if !self.linked && self.node.is_some() {
            let capture = graph
                .iter()
                .find(|v| {
                    v["info"]["props"]["node.name"] == "linmic_monitor_capture"
                        && client.is_some()
                        && v["info"]["props"]["client.id"].as_u64() == client
                })
                .and_then(|v| v["id"].as_u64());
            let source = graph
                .iter()
                .find(|v| {
                    v["type"] == "PipeWire:Interface:Node"
                        && v["info"]["props"]["node.name"].as_str() == Some(&self.source)
                })
                .and_then(|v| v["id"].as_u64());
            let port = |node: Option<u64>, direction: &str| {
                graph
                    .iter()
                    .find(|v| {
                        node.is_some()
                            && v["type"] == "PipeWire:Interface:Port"
                            && v["info"]["props"]["node.id"].as_u64() == node
                            && v["info"]["props"]["port.direction"] == direction
                    })
                    .and_then(|v| v["id"].as_u64())
            };
            if let (Some(output), Some(input)) = (port(source, "out"), port(capture, "in")) {
                let status = Command::new("pw-link")
                    .args([output.to_string(), input.to_string()])
                    .stdout(Stdio::null())
                    .stderr(Stdio::null())
                    .status()?;
                if !status.success() {
                    bail!("Monitor connection unavailable");
                }
                self.linked = true;
            }
        }
        if !self.volume_ready() && self.started.is_some_and(|t| t.elapsed().as_secs() >= 3) {
            bail!("Monitor initialization timed out");
        }
        Ok(())
    }
}
impl Drop for Monitor {
    fn drop(&mut self) {
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stop_reaps_owned_process() {
        let child = Command::new("sleep").arg("30").spawn().unwrap();
        let pid = child.id();
        let mut monitor = Monitor::default();
        monitor.child = Some(child);
        assert!(monitor.running());
        monitor.stop();
        assert!(!monitor.running());
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
    #[test]
    #[ignore = "requires a live PipeWire session and LinMic source; monitoring stays at zero volume"]
    fn live_monitor_zero_volume_and_cleanup() {
        let source = std::env::var("LINMIC_TEST_SOURCE").unwrap_or_else(|_| "linmic_input".into());
        let mut monitor = Monitor::default();
        monitor.start(&source).unwrap();
        let pid = monitor.child.as_ref().unwrap().id();
        for _ in 0..25 {
            std::thread::sleep(std::time::Duration::from_millis(100));
            monitor.apply_volume().unwrap();
            if monitor.volume_ready() {
                break;
            }
        }
        assert!(monitor.volume_ready());
        let output = Command::new("wpctl")
            .args(["get-volume", &monitor.node.unwrap().to_string()])
            .output()
            .unwrap();
        assert!(String::from_utf8_lossy(&output.stdout).contains("0.00"));
        drop(monitor);
        assert!(!std::path::Path::new(&format!("/proc/{pid}")).exists());
    }
}
