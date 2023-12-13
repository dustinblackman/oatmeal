pub mod ollama;
pub mod openai;
use anyhow::bail;
use anyhow::Result;

use crate::domain::models::Backend;
use crate::domain::models::BackendName;

pub type BackendBox = Box<dyn Backend + Send + Sync>;

pub struct BackendManager {}

impl BackendManager {
    pub fn get(name: BackendName) -> Result<BackendBox> {
        if name == BackendName::Ollama {
            return Ok(Box::<ollama::Ollama>::default());
        }

        if name == BackendName::OpenAI {
            return Ok(Box::<openai::OpenAI>::default());
        }

        bail!(format!("No backend implemented for {name}"))
    }
}
