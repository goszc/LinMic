use serde_json::{json, Value};
use std::sync::mpsc::Sender;
#[derive(Debug)]
pub struct Tray {
    pub commands: Sender<Value>,
    pub windows: Sender<&'static str>,
    pub muted: bool,
}
impl ksni::Tray for Tray {
    fn id(&self) -> String {
        "org.linmic.LinMic".into()
    }
    fn title(&self) -> String {
        if self.muted {
            "LinMic — muted"
        } else {
            "LinMic"
        }
        .into()
    }
    fn icon_name(&self) -> String {
        if self.muted {
            "microphone-sensitivity-muted-symbolic"
        } else {
            "audio-input-microphone-symbolic"
        }
        .into()
    }
    fn activate(&mut self, _x: i32, _y: i32) {
        let _ = self.windows.send("open");
    }
    fn menu(&self) -> Vec<ksni::MenuItem<Self>> {
        use ksni::menu::StandardItem;
        let tr = crate::i18n::tr;
        vec![
            StandardItem {
                label: tr("Open LinMic", "Abrir LinMic"),
                activate: Box::new(|s: &mut Self| {
                    let _ = s.windows.send("open");
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: tr("Toggle mute", "Alternar mute"),
                activate: Box::new(|s: &mut Self| {
                    let _ = s.commands.send(json!({"type":"toggle-mute"}));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: tr("Disconnect", "Desconectar"),
                activate: Box::new(|s: &mut Self| {
                    let _ = s.commands.send(json!({"type":"disconnect"}));
                }),
                ..Default::default()
            }
            .into(),
            StandardItem {
                label: tr("Quit interface", "Sair da interface"),
                activate: Box::new(|s: &mut Self| {
                    let _ = s.windows.send("quit");
                }),
                ..Default::default()
            }
            .into(),
        ]
    }
}
