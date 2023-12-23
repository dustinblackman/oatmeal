#[cfg(test)]
#[path = "config_test.rs"]
mod tests;

use std::env;
use std::path;

use anyhow::bail;
use anyhow::Result;
use clap::ArgMatches;
use clap::Command;
use dashmap::DashMap;
use once_cell::sync::Lazy;
use strum::EnumIter;
use strum::EnumVariantNames;
use strum::IntoEnumIterator;
use tokio::fs;

use crate::domain::models::BackendName;
use crate::domain::models::EditorName;

static CONFIG: Lazy<DashMap<String, String>> = Lazy::new(DashMap::new);

#[derive(Clone, Copy, Eq, PartialEq, EnumIter, EnumVariantNames, strum::Display)]
#[strum(serialize_all = "kebab-case")]
pub enum ConfigKey {
    Backend,
    BackendHealthCheckTimeout,
    Editor,
    Model,
    ConfigFile,
    LangChainURL,
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

    pub fn default(key: ConfigKey) -> String {
        if key == ConfigKey::Username {
            let mut user = env::var("USER").unwrap_or_else(|_| return "".to_string());
            if user.is_empty() {
                user = "User".to_string();
            }

            return user;
        }

        let default_backend = BackendName::Ollama.to_string();
        let default_editor = EditorName::Clipboard.to_string();

        #[cfg(not(target_os = "macos"))]
        let config_path = dirs::cache_dir().unwrap().join("oatmeal/config.toml");
        #[cfg(target_os = "macos")]
        let config_path =
            path::PathBuf::from(env::var("HOME").unwrap()).join(".config/oatmeal/config.toml");

        let res = match key {
            ConfigKey::Backend => &default_backend,
            ConfigKey::BackendHealthCheckTimeout => "1000",
            ConfigKey::Editor => &default_editor,
            ConfigKey::Model => "",
            ConfigKey::LangChainURL => "http://localhost:8000",
            ConfigKey::OllamaURL => "http://localhost:11434",
            ConfigKey::OpenAiToken => "",
            ConfigKey::OpenAiURL => "https://api.openai.com",
            ConfigKey::Theme => "base16-onedark",
            ConfigKey::ThemeFile => "",

            // Special
            ConfigKey::ConfigFile => config_path.to_str().unwrap(),
            ConfigKey::SessionID => "",
            ConfigKey::Username => "",
        };

        return res.to_string();
    }

    pub async fn load(cmd: Command, clap_arg_matches: Vec<&ArgMatches>) -> Result<()> {
        for key in ConfigKey::iter() {
            Config::set(key, &Config::default(key))
        }

        let mut config_file = Config::default(ConfigKey::ConfigFile);
        for matches in clap_arg_matches.as_slice() {
            if let Some(arg_config_file) =
                matches.get_one::<String>(&ConfigKey::ConfigFile.to_string())
            {
                config_file = arg_config_file.to_string();
            }
        }

        let config_path = path::PathBuf::from(config_file);
        if config_path.exists() {
            let toml_str = fs::read_to_string(config_path).await?;
            let doc = toml_str.parse::<toml_edit::Document>()?;

            for key in ConfigKey::iter() {
                if let Some(val) = doc.get(&key.to_string()) {
                    // Use clap value parsers to do validation.
                    let mut possible_values = vec![];
                    if let Some(arg) = cmd
                        .get_arguments()
                        .find(|e| return e.get_long().unwrap() == key.to_string())
                    {
                        if !arg.get_possible_values().is_empty() {
                            possible_values = arg
                                .get_possible_values()
                                .iter()
                                .map(|e| return e.get_name().to_string())
                                .collect::<Vec<String>>();
                        }
                    }

                    if let Some(val_int) = val.as_integer() {
                        Config::set(key, &val_int.to_string());
                    } else if let Some(val_str) = val.as_str() {
                        if val_str.is_empty() {
                            continue;
                        }
                        if !possible_values.is_empty()
                            && !possible_values.contains(&val_str.to_string())
                        {
                            bail!(format!("config.toml has an invalid value for key '{key}': {val_str}\nPossible values are: {}", possible_values.join(", ")));
                        }
                        Config::set(key, val_str);
                    }
                }
            }
        }

        for key in ConfigKey::iter() {
            for matches in clap_arg_matches.as_slice() {
                if let Ok(Some(val)) = matches.try_get_one::<String>(&key.to_string()) {
                    if val.is_empty() {
                        continue;
                    }
                    Config::set(key, val)
                }
            }
        }

        tracing::debug!(
            username = Config::get(ConfigKey::Username),
            backend = Config::get(ConfigKey::Backend),
            editor = Config::get(ConfigKey::Editor),
            model = Config::get(ConfigKey::Model),
            theme = Config::get(ConfigKey::Theme),
            theme_file = Config::get(ConfigKey::ThemeFile),
            "config"
        );

        return Ok(());
    }

    pub fn serialize_default(cmd: Command) -> String {
        let toml_str = ConfigKey::iter()
            .filter_map(|key| {
                if key == ConfigKey::SessionID || key == ConfigKey::ConfigFile {
                    return None;
                }

                if key == ConfigKey::Username {
                    return Some(
                        "# Your user name displayed in all chat bubbles.\n# username = \"\""
                            .to_string(),
                    );
                }

                let arg = cmd
                    .get_arguments()
                    .find(|e| return e.get_long().unwrap() == key.to_string())
                    .unwrap();

                let mut description = arg.get_help().unwrap().to_string();

                description = description
                    .split("[default:")
                    .next()
                    .unwrap()
                    .trim()
                    .to_string();

                if !arg.get_possible_values().is_empty() {
                    let possible_values = arg
                        .get_possible_values()
                        .iter()
                        .map(|e| return e.get_name())
                        .collect::<Vec<_>>()
                        .join(", ");
                    description = format!("{description} [possible values: {}]", possible_values);
                }

                let mut val = Config::default(key);
                if val.is_empty() {
                    val = format!("# {key} = \"\"");
                } else if val.parse::<i32>().is_ok() {
                    val = format!("{key} = {val}");
                } else {
                    val = format!("{key} = \"{val}\"");
                }

                return Some(format!("# {description}\n{val}"));
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        return toml_str;
    }
}
