use anyhow::Result;
use clap::{Parser, Subcommand};
use serde_json::json;
#[derive(Parser)]
#[command(version, about = "Control LinMic without opening a GUI")]
struct Args {
    #[command(subcommand)]
    command: Command,
}
#[derive(Subcommand)]
enum Command {
    Status {
        #[arg(long)]
        json: bool,
    },
    Stats {
        #[arg(long)]
        watch: bool,
    },
    Devices,
    Mute,
    Unmute,
    ToggleMute,
    Disconnect,
    Pair,
    Forget {
        id: String,
    },
    PipewireInfo,
    Config {
        #[command(subcommand)]
        command: ConfigCommand,
    },
}
#[derive(Subcommand)]
enum ConfigCommand {
    Show,
    Set { key: String, value: String },
}
fn main() -> Result<()> {
    let a = Args::parse();
    let watch = matches!(a.command, Command::Stats { watch: true });
    let v = match a.command {
        Command::Status { .. } => json!({"type":"status"}),
        Command::Stats { .. } => json!({"type":"stats"}),
        Command::Devices => json!({"type":"devices"}),
        Command::Mute => json!({"type":"mute"}),
        Command::Unmute => json!({"type":"unmute"}),
        Command::ToggleMute => json!({"type":"toggle-mute"}),
        Command::Disconnect => json!({"type":"disconnect"}),
        Command::Pair => json!({"type":"pair"}),
        Command::Forget { id } => json!({"type":"forget","id":id}),
        Command::PipewireInfo => json!({"type":"pipewire-info"}),
        Command::Config {
            command: ConfigCommand::Show,
        } => json!({"type":"config-show"}),
        Command::Config {
            command: ConfigCommand::Set { key, value },
        } => json!({"type":"config-set","key":key,"value":value}),
    };
    loop {
        let response = linmic_ipc::request(v.clone())?;
        if let Some(e) = response.get("error") {
            anyhow::bail!("{e}")
        }
        println!("{}", serde_json::to_string_pretty(&response)?);
        if !watch {
            break;
        }
        std::thread::sleep(std::time::Duration::from_secs(1));
    }
    Ok(())
}
