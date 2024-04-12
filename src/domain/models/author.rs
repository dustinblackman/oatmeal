use std::fmt;

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

impl fmt::Display for Author {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let name = match self {
            Author::User => Config::get(ConfigKey::Username),
            Author::Oatmeal => String::from("Oatmeal"),
            Author::Model => Config::get(ConfigKey::Model),
        };
        return write!(f, "{}", name);
    }
}
