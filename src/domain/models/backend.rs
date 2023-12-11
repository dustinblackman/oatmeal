#[cfg(test)]
#[path = "backend_test.rs"]
mod tests;

use anyhow::Result;
use async_trait::async_trait;
use tokio::sync::mpsc;

use super::Author;
use super::EditorContext;
use super::Event;

pub struct BackendPrompt {
    pub text: String,
    pub backend_context: String,
    pub system_prompt: String,
}

impl BackendPrompt {
    pub fn new(text: String, backend_context: String) -> BackendPrompt {
        return BackendPrompt {
            text,
            backend_context,
            // TODO configurable?
            // TODO disable completely?
            system_prompt: "You're an assistant. You return any code examples in individual markdown code block, with the language added to the code block.".to_string()
        };
    }

    pub fn append_editor_context(&mut self, editor_context: &Option<EditorContext>) {
        if let Some(context) = editor_context {
            let lang = &context.language;
            let code = &context.code;

            let system_prompt = format!(". The coding language is {lang}.");
            self.text += &system_prompt;

            if !code.is_empty() {
                let code_prompt = format!(" The code is the following:\n{code}");
                self.text += &code_prompt;
            }
        }
    }
}

#[derive(Clone)]
pub struct BackendResponse {
    pub author: Author,
    pub text: String,
    pub done: bool,
    pub context: Option<String>,
}

#[async_trait]
pub trait Backend {
    /// Used at startup to verify all configurations are available to work with
    /// the backend.
    async fn health_check(&self) -> Result<()>;

    /// Called when using the `/modellist` slash commands to provide all
    /// available models for the backend.
    async fn list_models<'a>(&'a self) -> Result<Vec<String>>;

    /// Requests completions from the backend. Completion results may be
    /// streamed back to the UI by passing each should through a channel.
    ///
    /// Upon receiving all results, a final `done` boolean
    /// is provided as the last message to the channel.
    ///
    /// In order for a backend to maintain history, a context array must be
    /// provided by the backend. This can be passed alongside the `done`
    /// boolean, and will be provided on the next prompt to the backend.
    async fn get_completion<'a>(
        &self,
        prompt: BackendPrompt,
        tx: &'a mpsc::UnboundedSender<Event>,
    ) -> Result<()>;
}
