use dashmap::DashMap;
use once_cell::sync::Lazy;

static CONFIG: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

#[derive(PartialEq, Eq)]
pub enum ConfigKey {
    Backend,
    Editor,
    Model,
    OllamaURL,
    OpenAIToken,
    OpenAIURL,
    SessionID,
    Theme,
    ThemeFile,
    Username,
}

impl ToString for ConfigKey {
    fn to_string(&self) -> String {
        match self {
            ConfigKey::Backend => return String::from("backend"),
            ConfigKey::Editor => return String::from("editor"),
            ConfigKey::Model => return String::from("model"),
            ConfigKey::OllamaURL => return String::from("ollama-url"),
            ConfigKey::OpenAIToken => return String::from("openai-token"),
            ConfigKey::OpenAIURL => return String::from("openai-url"),
            ConfigKey::SessionID => return String::from("session-id"),
            ConfigKey::Theme => return String::from("theme"),
            ConfigKey::ThemeFile => return String::from("themefile"),
            ConfigKey::Username => return String::from("username"),
        }
    }
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
