#[cfg(test)]
#[path = "githubcopilot_test.rs"]
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

use self::githubcopilot_auth::GithubAuth;

pub mod githubcopilot_auth;

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

#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
struct CompletionRequest {
    model: String,
    messages: Vec<MessageRequest>,
    stream: bool,
    intent: bool,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionDeltaResponse {
    content: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionChoiceResponse {
    delta: CompletionDeltaResponse,
    finish_reason: Option<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    choices: Vec<CompletionChoiceResponse>,
}

pub struct GithubCopilot {
    url: String,
    timeout: String,
}

impl Default for GithubCopilot {
    fn default() -> GithubCopilot {
        return GithubCopilot {
            url: "https://api.githubcopilot.com".to_string(),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for GithubCopilot {
    fn name(&self) -> BackendName {
        return BackendName::GitHubCopilot;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("GithubCopilot URL is not defined");
        }

        // OpenAi are trolls with their API where the index either returns a 404 or a
        // 418. If using the official API, don't bother health checking it.
        if self.url == "https://api.githubcopilot.com" {
            return Ok(());
        }

        let res = reqwest::Client::new()
            .get(&self.url)
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "GithubCopilot is not reachable");
            bail!("GithubCopilot is not reachable");
        }

        let status = res.unwrap().status().as_u16();
        if status >= 400 {
            tracing::error!(status = status, "GithubCopilot health check failed");
            bail!("GithubCopilot health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let models: Vec<String> = vec!["gpt-4".to_string()];
        return Ok(models);
    }

    #[allow(clippy::implicit_return)]
    async fn get_completion<'a>(
        &self,
        prompt: BackendPrompt,
        tx: &'a mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let auth = GithubAuth::new(tx).await?;

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
            intent: true,
            messages: messages.clone(),
            stream: true,
        };

        let res = reqwest::Client::new()
            .post(format!("{url}/chat/completions", url = self.url))
            .header(
                "Authorization",
                format!("Bearer {token}", token = auth.token),
            )
            .header("content-type", "application/json")
            .header("x-request-id", uuid::Uuid::new_v4().to_string())
            .header("vscode-sessionid", auth.vscode_sessionid)
            .header("machine-id", auth.machine_id)
            .header("user-agent", "GitHubCopilotChat/0.4.1")
            .header("editor-version", "vscode/1.85.1")
            .header("editor-plugin-version", "copilot-chat/0.4.1")
            .header("openai-organization", "github-copilot")
            .header("openai-intent", "conversation-panel")
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to GithubCopilot"
            );
            bail!(format!(
                "Failed to make completion request to GithubCopilot {}",
                res.text().await?
            ));
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
            if ores.choices.is_empty() {
                continue;
            }

            let choice = &ores.choices[0];
            tracing::debug!(body = ?ores, "Completion response");
            if choice.finish_reason.is_some() {
                break;
            }
            if choice.delta.content.is_none() {
                continue;
            }

            let text = choice.delta.content.clone().unwrap().to_string();
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
            role: "system".to_string(),
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
