use std::{collections::HashMap, sync::OnceLock};
pub const LANGUAGES: [&str; 6] = ["pt-BR", "en", "es", "ru", "zh-CN", "ja"];
pub fn language() -> &'static str {
    static LANG: OnceLock<String> = OnceLock::new();
    LANG.get_or_init(|| {
        let raw = std::env::var("LINMIC_LANG")
            .ok()
            .or_else(|| {
                std::fs::read_to_string(linmic_common::config_path().with_file_name("ui-language"))
                    .ok()
            })
            .unwrap_or_else(|| std::env::var("LANG").unwrap_or_else(|_| "en".into()));
        let code = raw.trim().to_lowercase();
        if code.starts_with("pt") {
            "pt-BR"
        } else if code.starts_with("es") {
            "es"
        } else if code.starts_with("ru") {
            "ru"
        } else if code.starts_with("zh") {
            "zh-CN"
        } else if code.starts_with("ja") {
            "ja"
        } else {
            "en"
        }
        .into()
    })
}
pub fn tr(en: &str, pt: &str) -> String {
    static MESSAGES: OnceLock<HashMap<String, String>> = OnceLock::new();
    let map = MESSAGES.get_or_init(|| {
        serde_json::from_str(match language() {
            "pt-BR" => include_str!("../locales/pt-BR.json"),
            "es" => include_str!("../locales/es.json"),
            "ru" => include_str!("../locales/ru.json"),
            "zh-CN" => include_str!("../locales/zh-CN.json"),
            "ja" => include_str!("../locales/ja.json"),
            _ => include_str!("../locales/en.json"),
        })
        .expect("embedded locale is valid")
    });
    map.get(en)
        .cloned()
        .unwrap_or_else(|| if language() == "pt-BR" { pt } else { en }.into())
}
#[cfg(test)]
mod tests {
    #[test]
    fn locale_keys_match() {
        let sources = [
            include_str!("../locales/en.json"),
            include_str!("../locales/pt-BR.json"),
            include_str!("../locales/es.json"),
            include_str!("../locales/ru.json"),
            include_str!("../locales/zh-CN.json"),
            include_str!("../locales/ja.json"),
        ];
        let first: std::collections::BTreeMap<String, String> =
            serde_json::from_str(sources[0]).unwrap();
        for source in sources {
            let map: std::collections::BTreeMap<String, String> =
                serde_json::from_str(source).unwrap();
            assert_eq!(
                first.keys().collect::<Vec<_>>(),
                map.keys().collect::<Vec<_>>()
            );
            assert!(map.values().all(|v| !v.is_empty()));
        }
    }
}
