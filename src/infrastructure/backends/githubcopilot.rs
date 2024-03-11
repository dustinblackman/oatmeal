#[cfg(test)]
#[path = "githubcopilot_test.rs"]
mod tests;

use rand::Rng;
use std::io::Read;
use std::iter;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

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

fn generate_hex_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(length).collect()
}

fn get_cached_oauth_token() -> Result<String> {
    let home_dir = std::env::var("HOME").expect("HOME environment variable is not set");
    let file_path = Path::new(&home_dir).join(".config/github-copilot/hosts.json");
    let mut file = std::fs::OpenOptions::new()
        .read(true)
        .open(file_path.clone())?;

    let mut contents = String::new();
    let _ = file.read_to_string(&mut contents);

    let json_data: HostData = serde_json::from_str(&contents)?;

    if let Some(github_com) = json_data.github_com {
        if let Some(token) = github_com.oauth_token {
            return Ok(token);
        }
    }
    bail!("Github Copilot token not found")
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct HostData {
    #[serde(rename(serialize = "github.com", deserialize = "github.com"))]
    github_com: Option<GithubCom>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubCom {
    user: Option<String>,
    oauth_token: Option<String>,
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
    machine_id: String,
    vscode_sessionid: String,
    oauth_token: Option<String>,
    token: Option<Token>,
}

pub struct Token {
    token: String,
    expires_at: u64,
}

impl Default for GithubCopilot {
    fn default() -> GithubCopilot {
        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();

        let oauth_token = match get_cached_oauth_token() {
            Ok(token) => Some(token),
            Err(_) => None,
        };

        return GithubCopilot {
            url: "https://api.githubcopilot.com".to_string(),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
            vscode_sessionid: uuid::Uuid::new_v4().to_string() + &time,
            machine_id: generate_hex_string(65),
            oauth_token,
            token: None,
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

        if self.oauth_token.is_none() {
            bail!("GithubCopilot token not found. Please install copilot if needed: gh extension install github/gh-copilot and login using gh auth login");
        }

        // Same as with OpenAI backend
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
        let token = self.get_copilot_token().await?;

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
            .header("Authorization", format!("Bearer {token}", token = token))
            .header("content-type", "application/json")
            .header("x-request-id", uuid::Uuid::new_v4().to_string())
            .header("vscode-sessionid", &self.vscode_sessionid)
            .header("machine-id", &self.machine_id)
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: u64,
    chat_enabled: bool,
}

impl GithubCopilot {
    async fn get_copilot_token(&self) -> Result<String> {
        // check lifetime and return saved token if it's still valid
        match &self.token {
            Some(token) => {
                if token.token != ""
                    && token.expires_at
                        > SystemTime::now()
                            .duration_since(UNIX_EPOCH)
                            .unwrap()
                            .as_secs()
                {
                    return Ok(token.token.clone());
                }
            }
            None => {}
        }
        match self.create_new_token().await {
            Ok(token) => {
                // self.token = Some(Token {
                //     token: token.token.clone(),
                //     expires_at: token.expires_at,
                // });
                return Ok(token.token);
            }
            Err(err) => {
                bail!(err);
            }
        }
    }

    async fn create_new_token(&self) -> Result<CopilotTokenResponse> {
        let oauth_token = match &self.oauth_token {
            Some(token) => token,
            None => bail!("Github Copilot token not found"),
        };

        let res = reqwest::Client::new()
            .get("https://api.github.com/copilot_internal/v2/token".to_string())
            .header(
                "Authorization",
                format!("token {oauth_token}", oauth_token = oauth_token),
            )
            .header("editor-version", "vscode/1.85.1")
            .header("editor-plugin-version", "copilot-chat/0.4.1")
            .header("user-agent", "GitHubCopilotChat/0.4.1")
            .send()
            .await;

        let text = res?.text().await?;
        let token_result: CopilotTokenResponse = serde_json::from_str(&text)?;
        return Ok(token_result);
    }
}
