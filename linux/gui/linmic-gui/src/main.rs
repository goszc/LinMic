mod i18n;
mod monitor;
mod tray;
use gtk::{glib, prelude::*};
use ksni::blocking::TrayMethods;
use serde_json::{json, Value};
use std::{cell::RefCell, rc::Rc, sync::mpsc, time::Duration};
fn main() -> glib::ExitCode {
    let app = gtk::Application::builder()
        .application_id("org.linmic.LinMic")
        .build();
    app.connect_activate(|app| {
        if let Some(window) = app.windows().first() {
            window.present();
        } else {
            build(app);
        }
    });
    app.run()
}
fn build(app: &gtk::Application) {
    let tr = i18n::tr;
    let window = gtk::ApplicationWindow::builder()
        .application(app)
        .title("LinMic")
        .default_width(570)
        .default_height(790)
        .build();
    let css = gtk::CssProvider::new();
    css.load_from_data("window { background: #101e1a; color: #e8f6ef; } .title-1 { font-size: 34px; font-weight: 800; } .card { background: #1a3027; border: 1px solid #345144; border-radius: 20px; padding: 20px; } button { background: #29463a; color: #e8f6ef; border-radius: 12px; padding: 12px; } button.suggested-action { background: #087c61; color: white; } .hero { font-size: 22px; font-weight: 700; } .dim-label { color: #aac4b7; opacity: 1; } scale trough highlight { background: #77dfb5; } stackswitcher button:checked { background: #27523f; color: white; }");
    gtk::style_context_add_provider_for_display(
        &gtk::gdk::Display::default().expect("display"),
        &css,
        gtk::STYLE_PROVIDER_PRIORITY_APPLICATION,
    );
    let content = gtk::Box::new(gtk::Orientation::Vertical, 14);
    content.set_margin_top(24);
    content.set_margin_bottom(24);
    content.set_margin_start(24);
    content.set_margin_end(24);
    let title = gtk::Label::new(Some("LinMic"));
    title.add_css_class("title-1");
    title.set_xalign(0.);
    content.append(&title);
    let tagline = gtk::Label::new(Some(&tr(
        "Your voice. Your computer.",
        "Sua voz. Seu computador.",
    )));
    tagline.set_xalign(0.);
    tagline.add_css_class("dim-label");
    content.append(&tagline);
    let stack = gtk::Stack::new();
    stack.set_transition_type(gtk::StackTransitionType::Crossfade);
    let tabs = gtk::StackSwitcher::new();
    tabs.set_stack(Some(&stack));
    tabs.set_halign(gtk::Align::Fill);
    content.append(&tabs);
    let mic_page = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let connection_page = gtk::Box::new(gtk::Orientation::Vertical, 16);
    let health_page = gtk::Box::new(gtk::Orientation::Vertical, 16);
    stack.add_titled(&mic_page, Some("mic"), &tr("Microphone", "Microfone"));
    stack.add_titled(
        &connection_page,
        Some("connection"),
        &tr("Connection", "Conexão"),
    );
    stack.add_titled(
        &health_page,
        Some("health"),
        &tr("Diagnostics", "Diagnóstico"),
    );
    let scroll = gtk::ScrolledWindow::new();
    scroll.set_vexpand(true);
    scroll.set_child(Some(&stack));
    content.append(&scroll);
    stack.set_vhomogeneous(false);
    let hero = gtk::Box::new(gtk::Orientation::Vertical, 14);
    hero.add_css_class("card");
    mic_page.append(&hero);
    let status = gtk::Label::new(Some(&tr("Connecting to daemon…", "Conectando ao daemon…")));
    status.set_xalign(0.);
    status.set_wrap(true);
    status.add_css_class("hero");
    hero.append(&status);
    let waveform = gtk::DrawingArea::new();
    waveform.set_content_height(100);
    let data = Rc::new(RefCell::new(vec![0.; 32]));
    let d = data.clone();
    waveform.set_draw_func(move |_, cr, w, h| {
        cr.set_source_rgb(0.0, 0.65, 0.52);
        cr.set_line_width(4.);
        let values = d.borrow();
        for (i, &v) in values.iter().enumerate() {
            let x = (i as f64 + 0.5) * w as f64 / 32.;
            let a = (v as f64 * h as f64 / 2.).max(1.);
            cr.move_to(x, h as f64 / 2. - a);
            cr.line_to(x, h as f64 / 2. + a);
        }
        let _ = cr.stroke();
    });
    hero.append(&waveform);
    let level = gtk::Label::new(None);
    level.add_css_class("hero");
    hero.append(&level);
    let quick_metrics = gtk::Label::new(None);
    quick_metrics.add_css_class("dim-label");
    hero.append(&quick_metrics);
    let mute = gtk::Button::with_label(&tr("Mute microphone", "Mutar microfone"));
    mute.add_css_class("suggested-action");
    mute.set_height_request(48);
    hero.append(&mute);
    let metrics = gtk::Label::new(None);
    metrics.set_xalign(0.);
    metrics.set_wrap(true);
    metrics.set_selectable(true);
    health_page.append(&metrics);
    let gain_label = gtk::Label::new(Some(&tr("Receive gain (dB)", "Ganho de recepção (dB)")));
    gain_label.set_xalign(0.);
    let gain_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    gain_card.add_css_class("card");
    mic_page.append(&gain_card);
    gain_card.append(&gain_label);
    let gain = gtk::Scale::with_range(gtk::Orientation::Horizontal, -60., 24., 1.);
    gain.set_value(0.);
    gain.set_draw_value(true);
    gain_card.append(&gain);
    let presets = gtk::Box::new(gtk::Orientation::Horizontal, 8);
    for (label, value) in [("0 dB", 0.), ("+6 dB", 6.), ("+12 dB", 12.)] {
        let button = gtk::Button::with_label(label);
        button.set_hexpand(true);
        let slider = gain.clone();
        button.connect_clicked(move |_| slider.set_value(value));
        presets.append(&button);
    }
    gain_card.append(&presets);
    let monitor_card = gtk::Box::new(gtk::Orientation::Vertical, 10);
    monitor_card.add_css_class("card");
    mic_page.append(&monitor_card);
    let listen = gtk::CheckButton::with_label(&tr("Hear my microphone", "Ouvir meu microfone"));
    monitor_card.append(&listen);
    let monitor_note = gtk::Label::new(Some(&tr("Use headphones to avoid feedback. Plays through the default PC output. Off when disconnected or the interface exits.", "Use fones para evitar microfonia. Reproduz na saída padrão do PC. Desliga ao desconectar ou encerrar a interface.")));
    monitor_note.set_wrap(true);
    monitor_note.set_xalign(0.);
    monitor_note.add_css_class("dim-label");
    monitor_card.append(&monitor_note);
    let monitor_volume_label =
        gtk::Label::new(Some(&tr("Monitor volume (%)", "Volume do retorno (%)")));
    monitor_volume_label.set_xalign(0.);
    monitor_card.append(&monitor_volume_label);
    let monitor_volume = gtk::Scale::with_range(gtk::Orientation::Horizontal, 0., 100., 1.);
    monitor_volume.set_value(25.);
    monitor_volume.set_draw_value(true);
    monitor_volume.set_sensitive(false);
    monitor_card.append(&monitor_volume);
    let monitor_status = gtk::Label::new(None);
    monitor_status.set_wrap(true);
    monitor_status.set_xalign(0.);
    monitor_card.append(&monitor_status);
    let pair = gtk::Button::with_label(&tr("Pair a phone", "Parear telefone"));
    connection_page.append(&pair);
    let devices = gtk::Button::with_label(&tr("Paired devices", "Dispositivos pareados"));
    connection_page.append(&devices);
    let disconnect = gtk::Button::with_label(&tr("Disconnect", "Desconectar"));
    content.append(&disconnect);
    let note=gtk::Label::new(Some(&tr("Closing this window keeps the microphone running. Configure global shortcuts with linmic toggle-mute.","Fechar esta janela mantém o microfone ativo. Configure atalhos globais com linmic toggle-mute.")));
    note.set_wrap(true);
    note.add_css_class("dim-label");
    health_page.append(&note);
    let language_label = gtk::Label::new(Some(&tr("Language", "Idioma")));
    language_label.set_xalign(0.);
    health_page.append(&language_label);
    let languages = gtk::DropDown::from_strings(&[
        "Português (Brasil)",
        "English",
        "Español",
        "Русский",
        "简体中文",
        "日本語",
    ]);
    languages.set_selected(
        i18n::LANGUAGES
            .iter()
            .position(|v| *v == i18n::language())
            .unwrap_or(1) as u32,
    );
    let weak = window.downgrade();
    languages.connect_selected_notify(move |list| {
        if let Some(code) = i18n::LANGUAGES.get(list.selected() as usize) {
            if let Err(e) = linmic_common::private_write(
                &linmic_common::config_path().with_file_name("ui-language"),
                code.as_bytes(),
            ) {
                eprintln!("Language preference: {e}");
                return;
            }
            if let Some(window) = weak.upgrade() {
                let dialog = gtk::MessageDialog::builder()
                    .transient_for(&window)
                    .modal(true)
                    .buttons(gtk::ButtonsType::Close)
                    .text(i18n::tr(
                        "Restart the interface to apply the language.",
                        "Reinicie a interface para aplicar o idioma.",
                    ))
                    .build();
                dialog.connect_response(|d, _| d.close());
                dialog.present();
            }
        }
    });
    health_page.append(&languages);
    window.set_child(Some(&content));
    let (tx, rx) = mpsc::channel::<Value>();
    let (window_tx, window_rx) = mpsc::channel();
    let tray = tray::Tray {
        commands: tx.clone(),
        windows: window_tx,
        muted: false,
    }
    .spawn()
    .ok();
    if tray.is_some() {
        window.connect_close_request(|w| {
            w.hide();
            glib::Propagation::Stop
        });
    }
    let application = app.downgrade();
    let mut last_state = String::new();
    let (events_tx, events_rx) = mpsc::sync_channel::<(String, Value)>(8);
    std::thread::spawn(move || {
        let mut monitor = monitor::Monitor::default();
        monitor.volume = 0.25;
        let mut source = String::new();
        let mut streaming = false;
        let mut volume_dirty = false;
        let mut monitor_error = String::new();
        loop {
            match rx.recv_timeout(Duration::from_millis(100)) {
                Ok(v) => {
                    let kind = v["type"].as_str().unwrap_or("").to_owned();
                    let response = if kind == "monitor" {
                        monitor_error.clear();
                        let result = if v["enabled"].as_bool() == Some(true) && streaming {
                            monitor.start(&source)
                        } else {
                            monitor.stop();
                            Ok(())
                        };
                        volume_dirty = true;
                        match result {
                            Ok(()) => json!({"monitor_active":monitor.running()}),
                            Err(e) => {
                                monitor_error = e.to_string();
                                json!({"monitor_error":monitor_error,"monitor_active":false})
                            }
                        }
                    } else if kind == "monitor-volume" {
                        monitor.volume = v["value"].as_f64().unwrap_or(0.25).clamp(0., 1.);
                        volume_dirty = true;
                        json!({"monitor_active":monitor.running()})
                    } else {
                        linmic_ipc::request(v).unwrap_or_else(|e| json!({"error":e.to_string()}))
                    };
                    if events_tx.send((kind, response)).is_err() {
                        break;
                    }
                }
                Err(mpsc::RecvTimeoutError::Timeout) => {
                    let mut response = linmic_ipc::request(json!({"type":"status"}))
                        .unwrap_or_else(|e| json!({"error":e.to_string()}));
                    source = response["node_name"].as_str().unwrap_or("").to_owned();
                    streaming = response["state"] == "STREAMING";
                    if !streaming {
                        monitor.stop();
                    }
                    if monitor.running() && volume_dirty {
                        match monitor.apply_volume() {
                            Ok(()) => {
                                volume_dirty = !monitor.volume_ready();
                            }
                            Err(e) => {
                                monitor.stop();
                                monitor_error = e.to_string();
                            }
                        }
                    }
                    response["monitor_active"] = json!(monitor.running());
                    if !monitor_error.is_empty() {
                        response["monitor_error"] = json!(monitor_error);
                    }
                    if let Err(mpsc::TrySendError::Disconnected(_)) =
                        events_tx.try_send(("status".into(), response))
                    {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
    });
    let monitor_updating = Rc::new(RefCell::new(false));
    let mu = monitor_updating.clone();
    let t = tx.clone();
    listen.connect_toggled(move |button| {
        if !*mu.borrow() {
            let _ = t.send(json!({"type":"monitor","enabled":button.is_active()}));
        }
    });
    let t = tx.clone();
    monitor_volume.connect_value_changed(move |slider| {
        let _ = t.send(json!({"type":"monitor-volume","value":slider.value()/100.}));
    });
    let t = tx.clone();
    mute.connect_clicked(move |_| {
        let _ = t.send(json!({"type":"toggle-mute"}));
    });
    let t = tx.clone();
    disconnect.connect_clicked(move |_| {
        let _ = t.send(json!({"type":"disconnect"}));
    });
    let t = tx.clone();
    pair.connect_clicked(move |_| {
        let _ = t.send(json!({"type":"pair"}));
    });
    let t = tx.clone();
    devices.connect_clicked(move |_| {
        let _ = t.send(json!({"type":"devices"}));
    });
    let updating = Rc::new(RefCell::new(false));
    let u = updating.clone();
    let t = tx.clone();
    gain.connect_value_changed(move|slider|{if !*u.borrow(){let _=t.send(json!({"type":"config-set","key":"gain-db","value":format!("{:.0}",slider.value())}));}});
    let weak = window.downgrade();
    glib::timeout_add_local(Duration::from_millis(100), move || {
        let Some(window) = weak.upgrade() else {
            return glib::ControlFlow::Break;
        };
        while let Ok(action) = window_rx.try_recv() {
            if action == "open" {
                window.present();
            } else if let Some(app) = application.upgrade() {
                app.quit();
                return glib::ControlFlow::Break;
            }
        }
        while let Ok((kind, v)) = events_rx.try_recv() {
            if let Some(active) = v["monitor_active"].as_bool() {
                *monitor_updating.borrow_mut() = true;
                listen.set_active(active);
                *monitor_updating.borrow_mut() = false;
                monitor_volume.set_sensitive(active);
            }
            if let Some(error) = v["monitor_error"].as_str() {
                monitor_status.set_text(&format!(
                    "{}: {error}",
                    tr("Monitor unavailable", "Retorno indisponível")
                ));
            } else if v["monitor_active"].as_bool() == Some(true) {
                monitor_status.set_text(&tr("Local monitoring active", "Retorno local ativo"));
            } else {
                monitor_status.set_text("");
            }
            if let Some(error) = v.get("error") {
                status.set_text(&format!(
                    "{}: {error}",
                    tr("Daemon unavailable", "Daemon indisponível")
                ));
                continue;
            }
            if kind == "pair" {
                let dialog = gtk::Window::builder()
                    .transient_for(&window)
                    .modal(true)
                    .title(tr("Pair a phone", "Parear telefone"))
                    .default_width(380)
                    .resizable(false)
                    .build();
                let box_ = gtk::Box::new(gtk::Orientation::Vertical, 20);
                box_.set_margin_top(28);
                box_.set_margin_bottom(28);
                box_.set_margin_start(28);
                box_.set_margin_end(28);
                let instruction = gtk::Label::new(Some(&tr(
                    "Enter this code in LinMic on your phone",
                    "Digite este código no LinMic do celular",
                )));
                instruction.set_wrap(true);
                box_.append(&instruction);
                let code = gtk::Label::new(None);
                let digits = v["code"].as_str().unwrap_or("------");
                code.set_markup(&format!(
                    "<span size=\"34000\" weight=\"bold\">{} {}</span>",
                    &digits[..3],
                    &digits[3..]
                ));
                code.set_selectable(true);
                box_.append(&code);
                let countdown = gtk::Label::new(Some(&tr(
                    "Valid for 2 minutes · one use",
                    "Válido por 2 minutos · uso único",
                )));
                box_.append(&countdown);
                let close = gtk::Button::with_label(&tr("Close", "Fechar"));
                let weak = dialog.downgrade();
                close.connect_clicked(move |_| {
                    if let Some(w) = weak.upgrade() {
                        w.close();
                    }
                });
                box_.append(&close);
                let commands = tx.clone();
                dialog.connect_close_request(move |_| {
                    let _ = commands.send(json!({"type":"pair-cancel"}));
                    glib::Propagation::Proceed
                });
                let weak = dialog.downgrade();
                let started = std::time::Instant::now();
                glib::timeout_add_local(Duration::from_secs(1), move || {
                    if weak.upgrade().is_none() {
                        return glib::ControlFlow::Break;
                    }
                    let left = 120u64.saturating_sub(started.elapsed().as_secs());
                    countdown.set_text(&if left == 0 {
                        tr(
                            "Expired. Generate a new code.",
                            "Expirou. Gere um novo código.",
                        )
                    } else {
                        format!("{}: {} s", tr("Expires in", "Expira em"), left)
                    });
                    if left == 0 {
                        code.set_text("— — —");
                        glib::ControlFlow::Break
                    } else {
                        glib::ControlFlow::Continue
                    }
                });
                dialog.set_child(Some(&box_));
                dialog.present();
            } else if kind == "devices" {
                let dialog = gtk::Window::builder()
                    .transient_for(&window)
                    .modal(true)
                    .title(tr("Paired devices", "Dispositivos pareados"))
                    .default_width(360)
                    .build();
                let list = gtk::Box::new(gtk::Orientation::Vertical, 10);
                list.set_margin_top(20);
                list.set_margin_bottom(20);
                list.set_margin_start(20);
                list.set_margin_end(20);
                if let Some(devices) = v["devices"].as_array() {
                    for device in devices {
                        let row = gtk::Box::new(gtk::Orientation::Horizontal, 10);
                        row.append(&gtk::Label::new(device["name"].as_str()));
                        let forget = gtk::Button::with_label(&tr("Forget", "Esquecer"));
                        let id = device["id"].clone();
                        let t = tx.clone();
                        forget.connect_clicked(move |b| {
                            let _ = t.send(json!({"type":"forget","id":id}));
                            b.set_sensitive(false);
                        });
                        row.append(&forget);
                        list.append(&row);
                    }
                }
                dialog.set_child(Some(&list));
                dialog.present();
            } else if kind == "status" {
                let state = v["state"].as_str().unwrap_or("");
                level.set_text(&if state == "STREAMING" {
                    format!("{:.0} dB", v["peak_db"].as_f64().unwrap_or(-120.))
                } else {
                    "— dB".into()
                });
                quick_metrics.set_text(&if state == "STREAMING" {
                    format!(
                        "{:.0} ms · {} {:.1}%",
                        v["estimated_latency_ms"].as_f64().unwrap_or(0.),
                        tr("Loss", "Perda"),
                        v["loss_percent"].as_f64().unwrap_or(0.)
                    )
                } else {
                    tr("Pair a phone", "Parear telefone")
                });
                listen.set_sensitive(state == "STREAMING");
                mute.set_sensitive(state == "STREAMING");
                disconnect.set_sensitive(state != "IDLE");
                if !last_state.is_empty()
                    && state != last_state
                    && matches!(state, "IDLE" | "STREAMING")
                {
                    if let Some(app) = application.upgrade() {
                        let notification = gtk::gio::Notification::new("LinMic");
                        notification.set_body(Some(&format!(
                            "{} · {}",
                            tr(state, state),
                            v["phone"].as_str().unwrap_or("")
                        )));
                        app.send_notification(Some("connection"), &notification);
                    }
                }
                last_state = state.into();
                if let Some(ref tray) = tray {
                    let muted = v["muted"].as_bool().unwrap_or(false);
                    tray.update(|t| t.muted = muted);
                }
                status.set_text(&format!(
                    "{} · {}",
                    tr(
                        v["state"].as_str().unwrap_or(""),
                        v["state"].as_str().unwrap_or("")
                    ),
                    v["phone"].as_str().unwrap_or("LinMic")
                ));
                mute.set_label(&if v["muted"].as_bool().unwrap_or(false) {
                    tr("Unmute microphone", "Ativar microfone")
                } else {
                    tr("Mute microphone", "Mutar microfone")
                });
                metrics.set_text(&format!(
                    "{}: {:.1} ms\nRTT: {:.1} ms · {}: {:.1} ms\n{}: {:.2}% · {}: {} ms\nRMS: {:.1} dB · {}: {:.1} dB\n{} · Opus · 48 kHz · AEAD",
                    tr("Estimated latency", "Latência estimada"), v["estimated_latency_ms"].as_f64().unwrap_or(0.),
                    v["rtt_ms"].as_f64().unwrap_or(0.), tr("Jitter", "Jitter"), v["jitter_ms"].as_f64().unwrap_or(0.),
                    tr("Loss", "Perda"), v["loss_percent"].as_f64().unwrap_or(0.), tr("Buffer", "Buffer"), v["jitter_buffer_ms"],
                    v["rms_db"].as_f64().unwrap_or(-120.), tr("Peak", "Pico"), v["peak_db"].as_f64().unwrap_or(-120.),
                    tr(v["health"].as_str().unwrap_or(""), v["health"].as_str().unwrap_or(""))
                ));
                *updating.borrow_mut() = true;
                gain.set_value(v["gain_db"].as_f64().unwrap_or(0.));
                *updating.borrow_mut() = false;
                if let Some(a) = v["waveform"].as_array() {
                    *data.borrow_mut() =
                        a.iter().map(|v| v.as_f64().unwrap_or(0.) as f32).collect();
                    waveform.queue_draw();
                }
            }
        }
        glib::ControlFlow::Continue
    });
    window.present();
}
