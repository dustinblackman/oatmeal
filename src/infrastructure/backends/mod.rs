pub mod ollama;
pub mod openai;
use anyhow::bail;
use anyhow::Result;

use crate::domain::models::Backend;

pub type BackendBox = Box<dyn Backend + Send + Sync>;

pub struct BackendManager {}

impl BackendManager {
    pub fn get(name: &str) -> Result<BackendBox> {
        if name == "ollama" {
            return Ok(Box::<ollama::Ollama>::default());
        }

        if name == "openai" {
            return Ok(Box::<openai::OpenAI>::default());
        }

        bail!(format!("No backend implemented for {name}"))
    }
}
