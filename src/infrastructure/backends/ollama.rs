#[cfg(test)]
#[path = "ollama_test.rs"]
mod tests;

use std::time::Duration;

use anyhow::bail;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use serde::Deserialize;
use serde::Serialize;
use tokio::io::AsyncBufReadExt;
use tokio::sync::mpsc;
use tokio_util::io::StreamReader;

use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendName;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

fn convert_err(err: reqwest::Error) -> std::io::Error {
    let err_msg = err.to_string();
    return std::io::Error::new(std::io::ErrorKind::Interrupted, err_msg);
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionRequest {
    model: String,
    prompt: String,
    context: Option<Vec<i32>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    pub response: String,
    pub done: bool,
    pub context: Option<Vec<i32>>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Model {
    name: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ModelListResponse {
    pub models: Vec<Model>,
}

pub struct Ollama {
    url: String,
    timeout: String,
}

impl Default for Ollama {
    fn default() -> Ollama {
        return Ollama {
            url: Config::get(ConfigKey::OllamaURL),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for Ollama {
    fn name(&self) -> BackendName {
        return BackendName::Ollama;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        let res = reqwest::Client::new()
            .get(&self.url)
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "Ollama is not running");
            bail!("Ollama is not running");
        }

        let res = res.unwrap();
        if res.status() != 200 {
            tracing::error!(status = res.status().as_u16(), "Ollama health checkfailed");
            bail!("Ollama health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let res = reqwest::Client::new()
            .get(format!("{url}/api/tags", url = self.url))
            .send()
            .await?
            .json::<ModelListResponse>()
            .await?;

        let mut models: Vec<String> = res
            .models
            .iter()
            .map(|model| {
                return model.name.to_string();
            })
            .collect();

        models.sort();

        return Ok(models);
    }

    #[allow(clippy::implicit_return)]
    async fn get_completion<'a>(
        &self,
        prompt: BackendPrompt,
        tx: &'a mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let mut req = CompletionRequest {
            model: Config::get(ConfigKey::Model),
            prompt: prompt.text,
            context: None,
        };

        if !prompt.backend_context.is_empty() {
            req.context = Some(serde_json::from_str(&prompt.backend_context)?);
        }

        let res = reqwest::Client::new()
            .post(format!("{url}/api/generate", url = self.url))
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to Ollama"
            );
            bail!("Failed to make completion request to Ollama");
        }

        let stream = res.bytes_stream().map_err(convert_err);
        let mut lines_reader = StreamReader::new(stream).lines();

        while let Ok(line) = lines_reader.next_line().await {
            if line.is_none() {
                break;
            }

            let ores: CompletionResponse = serde_json::from_str(&line.unwrap()).unwrap();
            tracing::debug!(body = ?ores, "Completion response");
            let mut msg = BackendResponse {
                author: Author::Model,
                text: ores.response,
                done: ores.done,
                context: None,
            };
            if ores.done && ores.context.is_some() {
                msg.context = Some(serde_json::to_string(&ores.context)?);
            }

            tx.send(Event::BackendPromptResponse(msg))?;
        }

        return Ok(());
    }
}
