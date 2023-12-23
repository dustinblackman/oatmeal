#[cfg(test)]
#[path = "openai_test.rs"]
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
struct Model {
    id: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ModelListResponse {
    data: Vec<Model>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct MessageRequest {
    role: String,
    content: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<MessageRequest>,
    stream: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionDeltaResponse {
    content: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionChoiceResponse {
    delta: CompletionDeltaResponse,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoiceResponse>,
}

pub struct OpenAI {
    url: String,
    token: String,
    timeout: String,
}

impl Default for OpenAI {
    fn default() -> OpenAI {
        return OpenAI {
            url: Config::get(ConfigKey::OpenAiURL),
            token: Config::get(ConfigKey::OpenAiToken),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for OpenAI {
    fn name(&self) -> BackendName {
        return BackendName::OpenAI;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("OpenAI URL is not defined");
        }
        if self.token.is_empty() {
            bail!("OpenAI token is not defined");
        }

        // OpenAI are trolls with their API where the index either returns a 404 or a
        // 418. If using the official API, don't bother health checking it.
        if self.url == "https://api.openai.com" {
            return Ok(());
        }

        let res = reqwest::Client::new()
            .get(&self.url)
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "OpenAI is not reachable");
            bail!("OpenAI is not reachable");
        }

        let status = res.unwrap().status().as_u16();
        if status >= 400 {
            tracing::error!(status = status, "OpenAI health check failed");
            bail!("OpenAI health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let res = reqwest::Client::new()
            .get(format!("{url}/v1/models", url = self.url))
            .header("Authorization", format!("Bearer {}", self.token))
            .send()
            .await?
            .json::<ModelListResponse>()
            .await?;

        let mut models: Vec<String> = res
            .data
            .iter()
            .map(|model| {
                return model.id.to_string();
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
        let mut messages: Vec<MessageRequest> = vec![];
        if !prompt.backend_context.is_empty() {
            messages = serde_json::from_str(&prompt.backend_context)?;
        }
        messages.push(MessageRequest {
            role: "user".to_string(),
            content: prompt.text,
        });

        let req = CompletionRequest {
            model: Config::get(ConfigKey::Model),
            messages: messages.clone(),
            stream: true,
        };

        let res = reqwest::Client::new()
            .post(format!("{url}/v1/chat/completions", url = self.url))
            .header("Authorization", format!("Bearer {}", self.token))
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to OpenAI"
            );
            bail!("Failed to make completion request to OpenAI");
        }

        let stream = res.bytes_stream().map_err(convert_err);
        let mut lines_reader = StreamReader::new(stream).lines();

        let mut last_message = "".to_string();
        while let Ok(line) = lines_reader.next_line().await {
            if line.is_none() {
                break;
            }

            let mut cleaned_line = line.unwrap().trim().to_string();
            if cleaned_line.starts_with("data:") {
                cleaned_line = cleaned_line.split_off(5).trim().to_string();
            }
            if cleaned_line.is_empty() {
                continue;
            }

            let ores: CompletionResponse = serde_json::from_str(&cleaned_line).unwrap();
            tracing::debug!(body = ?ores, "Completion response");

            let text = ores.choices[0].delta.content.to_string();
            last_message += &text;
            let msg = BackendResponse {
                author: Author::Model,
                text,
                done: false,
                context: None,
            };

            tx.send(Event::BackendPromptResponse(msg))?;
        }

        messages.push(MessageRequest {
            role: "assistant".to_string(),
            content: last_message.to_string(),
        });

        let msg = BackendResponse {
            author: Author::Model,
            text: "".to_string(),
            done: true,
            context: Some(serde_json::to_string(&messages)?),
        };
        tx.send(Event::BackendPromptResponse(msg))?;

        return Ok(());
    }
}
