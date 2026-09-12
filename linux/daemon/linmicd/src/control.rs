use super::*;
use anyhow::{bail, ensure};
use linmic_network::{read_json, write_json};
use rand::{Rng, RngCore};
use std::net::{TcpListener, TcpStream};
fn save_devices(a: &App) -> Result<()> {
    linmic_common::private_write(
        &linmic_common::data_dir().join("devices.json"),
        &serde_json::to_vec_pretty(&a.devices)?,
    )
}
pub fn local(s: &Shared, v: Value) -> Result<Value> {
    let cmd = v["type"].as_str().unwrap_or("");
    if matches!(cmd, "status" | "stats" | "pipewire-info") {
        return Ok(s.status());
    }
    if cmd == "disconnect" {
        s.disconnect();
        return Ok(json!({"ok":true}));
    }
    let mut a = s.app.lock().unwrap();
    match cmd {
        "devices" => Ok(
            json!({"devices":a.devices.as_array().unwrap().iter().map(|d|json!({"id":d["id"],"name":d["name"]})).collect::<Vec<_>>()}),
        ),
        "forget" => {
            let id = v["id"].as_str().context("device id required")?;
            a.devices.as_array_mut().unwrap().retain(|d| d["id"] != id);
            save_devices(&a)?;
            if a.session.as_ref().is_some_and(|x| x.device == id) {
                a.session = None;
                a.state = State::Idle;
                s.silent.store(true, Ordering::Release);
                s.stats.generation.fetch_add(1, Ordering::AcqRel);
            }
            Ok(json!({"ok":true}))
        }
        "pair-cancel" => {
            a.pairing = None;
            Ok(json!({"ok":true}))
        }
        "pair" => {
            let code = format!("{:06}", rand::rngs::OsRng.gen_range(0..1_000_000));
            a.pairing = Some((code.clone(), Instant::now(), 0));
            Ok(
                json!({"code":code,"expires_seconds":120,"method":"spake2-v1","instruction":"Enter these six digits in the Android app."}),
            )
        }
        "mute" | "unmute" | "toggle-mute" => {
            a.muted = match cmd {
                "mute" => true,
                "unmute" => false,
                _ => !a.muted,
            };
            a.revision += 1;
            if a.session.as_ref().is_some_and(|x| x.started) {
                a.state = if a.muted {
                    State::Muted
                } else {
                    State::Streaming
                };
            }
            if a.muted {
                s.silent.store(true, Ordering::Release);
            }
            Ok(json!({"muted":a.muted,"revision":a.revision}))
        }
        "config-show" => Ok(serde_json::to_value(&a.config)?),
        "config-set" => {
            let key = v["key"].as_str().context("key required")?;
            let value = v["value"].as_str().context("value required")?;
            let mut config = a.config.clone();
            match key {
                "gain-db" => config.audio.gain_db = value.parse()?,
                "profile" => config.audio.default_profile = value.into(),
                "noise-gate" => config.audio.noise_gate = value.parse()?,
                "agc" => config.audio.agc = value.parse()?,
                "jitter-target" => config.jitter.target_ms = value.parse()?,
                _ => bail!("Supported settings: gain-db, profile, noise-gate, agc, jitter-target"),
            };
            config.save()?;
            a.config = config;
            Ok(json!({"ok":true}))
        }
        _ => bail!("unknown command"),
    }
}
pub fn serve(listener: TcpListener, identity: Arc<linmic_network::Identity>, s: Arc<Shared>) {
    let mut workers: Vec<std::thread::JoinHandle<()>> = Vec::new();
    while !s.stop.load(Ordering::Acquire) {
        workers.retain(|w| !w.is_finished());
        match listener.accept() {
            Ok((stream, _)) => {
                if workers.len() >= 4 {
                    drop(stream);
                    continue;
                }
                let s = s.clone();
                let i = identity.clone();
                workers.push(std::thread::spawn(move || {
                    if let Err(e) = client(stream, &i, &s) {
                        eprintln!("Control connection ended: {e}");
                    }
                }));
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50))
            }
            Err(e) => {
                eprintln!("Control accept: {e}");
                break;
            }
        }
    }
    for w in workers {
        let _ = w.join();
    }
}
fn client(socket: TcpStream, identity: &linmic_network::Identity, s: &Shared) -> Result<()> {
    let ip = socket.peer_addr()?.ip();
    ensure!(
        linmic_network::local_address(ip),
        "Only local/private networks are allowed"
    );
    let mut io = identity.accept(socket)?;
    let hello = read_json(&mut io)?;
    if hello["type"] != "hello" || hello["protocol"] != 1 {
        write_json(
            &mut io,
            &json!({"type":"error","code":"PROTOCOL_UNSUPPORTED","supported":[1]}),
        )?;
        bail!("unsupported hello")
    }
    let name = hello["client"]["name"]
        .as_str()
        .unwrap_or("Android")
        .chars()
        .filter(|c| !c.is_control())
        .take(64)
        .collect::<String>();
    let token = hello["token"].as_str().unwrap_or("");
    let mut device = {
        let a = s.app.lock().unwrap();
        a.devices
            .as_array()
            .unwrap()
            .iter()
            .find(|d| constant_equal(d["token"].as_str().unwrap_or(""), token) && token.len() == 64)
            .map(|d| d["id"].as_str().unwrap_or("").to_string())
    };
    if device.is_none() {
        let prepared = (|| -> Result<_> {
            let mut a = s.app.lock().unwrap();
            let (code, created, attempts) = a
                .pairing
                .as_mut()
                .context("Open pairing in the desktop app first")?;
            ensure!(
                created.elapsed() < Duration::from_secs(120) && *attempts < 5,
                "Pairing expired or locked; generate a new code on desktop"
            );
            *attempts += 1;
            let (exchange, message) =
                linmic_pairing::Exchange::start(code, true).map_err(anyhow::Error::msg)?;
            Ok((exchange, message, *created))
        })();
        let (exchange, message, issued) = match prepared {
            Ok(v) => v,
            Err(e) => {
                write_json(
                    &mut io,
                    &json!({"type":"pair_error","message":e.to_string()}),
                )?;
                return Err(e);
            }
        };
        write_json(
            &mut io,
            &json!({"type":"pair_challenge","method":"spake2-v1","message":hex::encode(message),"expires_seconds":120}),
        )?;
        let submit = read_json(&mut io)?;
        let result = (|| -> Result<(String, String, [u8; 32])> {
            ensure!(
                submit["type"] == "pair_submit" && submit["method"] == "spake2-v1",
                "Secure pairing requires an updated app"
            );
            let message = hex::decode(submit["message"].as_str().context("pair message")?)?;
            let key = exchange.finish(&message).map_err(anyhow::Error::msg)?;
            let certificate = hex::decode(&identity.fingerprint)?;
            let proof = hex::decode(submit["proof"].as_str().context("pair proof")?)?;
            ensure!(
                linmic_pairing::verify(&key, &certificate, false, &proof),
                "Pairing code incorrect"
            );
            let confirmation = linmic_pairing::proof(&key, &certificate, true);
            let mut a = s.app.lock().unwrap();
            ensure!(a.pairing.as_ref().is_some_and(|(_,t,_)|*t==issued&&t.elapsed()<Duration::from_secs(120)),"Pairing code expired or already used");
            ensure!(
                a.devices.as_array().unwrap().len() < 32,
                "device limit reached"
            );
            let token = linmic_network::random_hex();
            let id = linmic_network::random_hex();
            a.devices
                .as_array_mut()
                .unwrap()
                .push(json!({"id":id,"name":name,"token":token}));
            if let Err(e) = save_devices(&a) {
                a.devices.as_array_mut().unwrap().pop();
                return Err(e);
            }
            a.pairing = None;
            Ok((id, token, confirmation))
        })();
        match result {
            Ok((id, token, confirmation)) => {
                write_json(
                    &mut io,
                    &json!({"type":"pair_success","device_id":id,"device_token":token,"proof":hex::encode(confirmation)}),
                )?;
                device = Some(id);
            }
            Err(e) => {
                write_json(
                    &mut io,
                    &json!({"type":"pair_error","message":"Pairing failed. Check the code or generate a new one on desktop."}),
                )?;
                return Err(e);
            }
        }
    }
    let frame = linmic_common::profile(hello["profile"].as_str().unwrap_or("balanced"))?.0;
    let (id, key, port) = {
        let mut a = s.app.lock().unwrap();
        ensure!(
            a.session.is_none()
                || a.session.as_ref().unwrap().heartbeat.elapsed() > Duration::from_secs(5),
            "another phone is connected"
        );
        let id = rand::rngs::OsRng.gen_range(1..u32::MAX);
        let mut key = [0; 32];
        rand::rngs::OsRng.fill_bytes(&mut key);
        a.session = Some(Session {
            id,
            key,
            ip,
            name,
            device: device.unwrap(),
            frame,
            started: false,
            heartbeat: Instant::now(),
            created: Instant::now(),
        });
        a.state = State::Connected;
        s.silent.store(true, Ordering::Release);
        s.stats.generation.fetch_add(1, Ordering::AcqRel);
        s.stats.session_packets.store(0, Ordering::Relaxed);
        s.stats.session_lost.store(0, Ordering::Relaxed);
        (id, key, a.config.network.audio_port)
    };
    write_json(
        &mut io,
        &json!({"type":"hello_ack","protocol":1,"session_id":id,"udp_port":port,"audio_key":hex::encode(key),"frame_samples":frame}),
    )?;
    let result = (|| -> Result<()> {
        while !s.stop.load(Ordering::Acquire) {
            let v = read_json(&mut io)?;
            let received_us = linmic_common::monotonic_us();
            {
                let mut a = s.app.lock().unwrap();
                let sess = a
                    .session
                    .as_mut()
                    .filter(|x| x.id == id)
                    .context("session disconnected")?;
                sess.heartbeat = Instant::now();
            }
            let response = match v["type"].as_str().unwrap_or("") {
                "start_stream" => {
                    let mut a = s.app.lock().unwrap();
                    a.session.as_mut().unwrap().started = true;
                    a.state = if a.muted {
                        State::Muted
                    } else {
                        State::Streaming
                    };
                    json!({"type":"start_stream_ack","muted":a.muted,"revision":a.revision})
                }
                "ping" | "stats_client" => {
                    let rtt = v["rtt_ms"].as_f64().unwrap_or(0.).clamp(0., 10000.);
                    s.app.lock().unwrap().rtt_ms = rtt;
                    let mut stats = s.status();
                    stats["type"] = json!("pong");
                    stats["t0"] = v["t0"].clone();
                    stats["t1"] = json!(received_us);
                    stats["t2"] = json!(linmic_common::monotonic_us());
                    stats
                }
                "mute" | "unmute" => {
                    let mut a = s.app.lock().unwrap();
                    let rev = v["revision"].as_u64().unwrap_or(0);
                    if rev >= a.revision {
                        a.revision = rev.saturating_add(1);
                        a.muted = v["type"] == "mute";
                        a.state = if a.muted {
                            State::Muted
                        } else {
                            State::Streaming
                        };
                        if a.muted {
                            s.silent.store(true, Ordering::Release);
                        }
                    }
                    json!({"type":"mute","muted":a.muted,"revision":a.revision})
                }
                "set_gain" => {
                    let mut a = s.app.lock().unwrap();
                    let gain = v["gain_db"].as_f64().context("gain_db")?;
                    ensure!(
                        gain.is_finite() && (-60.0..=24.).contains(&gain),
                        "invalid gain"
                    );
                    a.config.audio.gain_db = gain as f32;
                    json!({"type":"set_gain","gain_db":gain})
                }
                "set_profile" => {
                    json!({"type":"error","code":"RECONNECT_REQUIRED","message":"Reconnect to negotiate a new frame size"})
                }
                "stop_stream" | "disconnect" => {
                    write_json(&mut io, &json!({"type":"stop_stream_ack"}))?;
                    return Ok(());
                }
                _ => json!({"type":"error","code":"UNKNOWN_MESSAGE"}),
            };
            write_json(&mut io, &response)?;
        }
        Ok(())
    })();
    {
        let mut a = s.app.lock().unwrap();
        if a.session.as_ref().is_some_and(|x| x.id == id) {
            a.session = None;
            a.state = State::Idle;
            s.silent.store(true, Ordering::Release);
            s.stats.generation.fetch_add(1, Ordering::AcqRel);
        }
    }
    result
}
fn constant_equal(a: &str, b: &str) -> bool {
    if a.len() != b.len() {
        return false;
    }
    a.bytes().zip(b.bytes()).fold(0u8, |v, (a, b)| v | (a ^ b)) == 0
}
