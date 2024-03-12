#[cfg(test)]
#[path = "claude_test.rs"]
mod tests;

use std::time::Duration;

use anyhow::bail;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use itertools::Itertools;
use regex::Regex;
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
    max_tokens: u32,
    messages: Vec<MessageRequest>,
    stream: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Healthcheck {
    message: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionDeltaResponse {
    #[serde(rename = "type")]
    _type: String,
    text: String,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    #[serde(rename = "type")]
    _type: String,
    delta: CompletionDeltaResponse,
}

pub struct Claude {
    url: String,
    token: String,
    timeout: String,
}

impl Default for Claude {
    fn default() -> Claude {
        return Claude {
            url: "https://api.anthropic.com".to_string(),
            token: Config::get(ConfigKey::ClaudeToken),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for Claude {
    fn name(&self) -> BackendName {
        return BackendName::Claude;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("Claude URL is not defined");
        }
        if self.token.is_empty() {
            bail!("Claude token is not defined");
        }

        let res = reqwest::Client::new()
            .get(format!("{url}/healthcheck", url = self.url))
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "Claude is not reachable");
            bail!("Claude is not reachable");
        }

        let result = res.unwrap();
        let status = result.status().as_u16();
        if status >= 400 {
            tracing::error!(status = status, "Claude health check failed");
            bail!("Claude health check failed");
        }

        let json = result.json::<Healthcheck>().await?;
        if !json.message.contains("ok") {
            bail!("Claude health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let res = reqwest::Client::new()
            .get("https://raw.githubusercontent.com/anthropics/anthropic-sdk-typescript/main/src/resources/messages.ts")
            .send()
            .await?
            .text()
            .await;

        let models: Vec<String> = match res {
            Ok(html) => {
                let re = Regex::new(r#"['"](claude-.*)['"]"#).unwrap();
                let mut results: Vec<String> = vec![];
                for (_, [model]) in re.captures_iter(&html).map(|c| c.extract()) {
                    let m = model.to_string();
                    let cleaned = Regex::new(r"[^a-zA-Z0-9-\.]")
                        .unwrap()
                        .replace_all(&m, "")
                        .to_string();
                    results.push(cleaned);
                }
                results.into_iter().unique().collect()
            }
            Err(_) => {
                vec![
                    "claude-3-sonnet-20240229".to_string(),
                    "claude-3-opus-20240229".to_string(),
                    "claude-2.1".to_string(),
                    "claude-2.0".to_string(),
                ]
            }
        };

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
            max_tokens: 1024,
            messages: messages.clone(),
            stream: true,
        };

        let res = reqwest::Client::new()
            .post(format!("{url}/v1/messages", url = self.url))
            .header("x-api-key", &self.token)
            .header("content-type", "application/json")
            .header("anthropic-version", "2023-06-01")
            .header("anthropic-beta", "messages-2023-12-15")
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to Claude"
            );
            bail!("Failed to make completion request to Claude");
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
            if cleaned_line.is_empty() || cleaned_line.contains("event:") {
                continue;
            }

            if cleaned_line.contains("content_block_stop") {
                break;
            }
            if !cleaned_line.contains("content_block_delta") {
                continue;
            }

            let ores: CompletionResponse = serde_json::from_str(&cleaned_line).unwrap();
            tracing::debug!(body = ?ores, "Completion response");

            let text = ores.delta.text.clone().to_string();
            if text.is_empty() {
                continue;
            }

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
