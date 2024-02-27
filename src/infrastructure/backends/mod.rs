pub mod githubcopilot;
pub mod langchain;
pub mod ollama;
pub mod openai;
use anyhow::bail;
use anyhow::Result;

use crate::domain::models::BackendBox;
use crate::domain::models::BackendName;

pub struct BackendManager {}

impl BackendManager {
    pub fn get(name: BackendName) -> Result<BackendBox> {
        if name == BackendName::LangChain {
            return Ok(Box::<langchain::LangChain>::default());
        }

        if name == BackendName::Ollama {
            return Ok(Box::<ollama::Ollama>::default());
        }

        if name == BackendName::OpenAI {
            return Ok(Box::<openai::OpenAI>::default());
        }

        if name == BackendName::GitHubCopilot {
            return Ok(Box::<githubcopilot::GithubCopilot>::default());
        }

        bail!(format!("No backend implemented for {name}"))
    }
}
