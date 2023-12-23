use serde::Deserialize;
use serde::Serialize;

use crate::configuration::Config;
use crate::configuration::ConfigKey;

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub enum Author {
    User,
    Oatmeal,
    Model,
}

impl ToString for Author {
    fn to_string(&self) -> String {
        match self {
            Author::User => return Config::get(ConfigKey::Username),
            Author::Oatmeal => return String::from("Oatmeal"),
            Author::Model => return Config::get(ConfigKey::Model),
        }
    }
}
