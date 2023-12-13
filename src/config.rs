use dashmap::DashMap;
use once_cell::sync::Lazy;
use strum::EnumVariantNames;

static CONFIG: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

#[derive(PartialEq, Eq, EnumVariantNames, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ConfigKey {
    Backend,
    BackendHealthCheckTimeout,
    Editor,
    Model,
    OllamaURL,
    OpenAiToken,
    OpenAiURL,
    SessionID,
    Theme,
    ThemeFile,
    Username,
}

pub struct Config {}

impl Config {
    pub fn get(key: ConfigKey) -> String {
        if let Some(val) = CONFIG.get(&key.to_string()) {
            return val.to_string();
        }

        return "".to_string();
    }

    pub fn set(key: ConfigKey, value: &str) {
        CONFIG.insert(key.to_string(), value.to_string());
    }
}
