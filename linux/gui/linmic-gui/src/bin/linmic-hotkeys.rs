//! Global shortcuts through the desktop portal; no keyboard interception.
use anyhow::{ensure, Context, Result};
use clap::Parser;
use std::collections::HashMap;
use zbus::{
    blocking::{Connection, Proxy},
    zvariant::{OwnedObjectPath, OwnedValue, Value},
};
#[derive(Parser)]
struct Args {
    #[arg(long,default_value="toggle",value_parser=["toggle","push-to-talk","push-to-mute"])]
    mode: String,
    #[arg(long, help = "Check portal support without requesting a shortcut")]
    check: bool,
}
const DEST: &str = "org.freedesktop.portal.Desktop";
fn request_path(connection: &Connection, token: &str) -> String {
    format!(
        "/org/freedesktop/portal/desktop/request/{}/{}",
        connection
            .unique_name()
            .unwrap()
            .as_str()
            .trim_start_matches(':')
            .replace('.', "_"),
        token
    )
}
fn main() -> Result<()> {
    let args = Args::parse();
    let connection = Connection::session()?;
    let portal = Proxy::new(
        &connection,
        DEST,
        "/org/freedesktop/portal/desktop",
        "org.freedesktop.portal.GlobalShortcuts",
    )?;
    let version:u32=portal.get_property("version").context("This desktop does not provide the GlobalShortcuts portal. Bind linmic toggle-mute in desktop settings instead.")?;
    if args.check {
        println!("GlobalShortcuts portal version {version}; toggle, push-to-talk and push-to-mute available");
        return Ok(());
    }
    let path = request_path(&connection, "linmic_create");
    let response = Proxy::new(
        &connection,
        DEST,
        path.as_str(),
        "org.freedesktop.portal.Request",
    )?;
    let mut signals = response.receive_signal("Response")?;
    let options = HashMap::from([
        ("handle_token", Value::from("linmic_create")),
        ("session_handle_token", Value::from("linmic_shortcuts")),
    ]);
    let _: OwnedObjectPath = portal.call("CreateSession", &(options,))?;
    let message = signals.next().context("portal closed")?;
    let (code, result): (u32, HashMap<String, OwnedValue>) = message.body().deserialize()?;
    ensure!(code == 0, "shortcut session denied");
    let session = OwnedObjectPath::try_from(
        result
            .get("session_handle")
            .context("missing session")?
            .try_clone()?,
    )?;
    let path = request_path(&connection, "linmic_bind");
    let response = Proxy::new(
        &connection,
        DEST,
        path.as_str(),
        "org.freedesktop.portal.Request",
    )?;
    let mut responses = response.receive_signal("Response")?;
    let shortcuts = vec![(
        "linmic-mute",
        HashMap::from([
            (
                "description",
                Value::from(format!("LinMic — {}", args.mode)),
            ),
            ("preferred_trigger", Value::from("CTRL+ALT+m")),
        ]),
    )];
    let options = HashMap::from([("handle_token", Value::from("linmic_bind"))]);
    let _: OwnedObjectPath = portal.call("BindShortcuts", &(&session, shortcuts, "", options))?;
    let response = responses.next().context("portal closed")?;
    let (code, _): (u32, HashMap<String, OwnedValue>) = response.body().deserialize()?;
    ensure!(code == 0, "shortcut permission denied");
    if args.mode == "push-to-talk" {
        linmic_ipc::request(serde_json::json!({"type":"mute"}))?;
    }
    println!(
        "LinMic {} shortcut active. Keep this process running.",
        args.mode
    );
    for signal in portal.receive_all_signals()? {
        let header = signal.header();
        let Some(member) = header.member() else {
            continue;
        };
        let active = match member.as_str() {
            "Activated" => true,
            "Deactivated" => false,
            _ => continue,
        };
        let (path, id, _, _): (OwnedObjectPath, String, u64, HashMap<String, OwnedValue>) =
            signal.body().deserialize()?;
        if path.as_str() != session.as_str() || id != "linmic-mute" {
            continue;
        }
        let command = match (args.mode.as_str(), active) {
            ("toggle", true) => "toggle-mute",
            ("toggle", false) => continue,
            ("push-to-talk", true) | ("push-to-mute", false) => "unmute",
            _ => "mute",
        };
        if let Err(e) = linmic_ipc::request(serde_json::json!({"type":command})) {
            eprintln!("Shortcut command failed: {e}");
        }
    }
    Ok(())
}
