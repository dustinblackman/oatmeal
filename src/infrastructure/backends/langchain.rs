#[cfg(test)]
#[path = "langchain_test.rs"]
mod tests;

use std::collections::HashMap;
use std::time::Duration;

use anyhow::anyhow;
use anyhow::bail;
use anyhow::Result;
use async_trait::async_trait;
use futures::stream::TryStreamExt;
use itertools::Itertools;
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
struct Empty {}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct OpenAPIJSONResponse {
    paths: HashMap<String, Empty>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionRequest {
    input: HashMap<String, String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionResponse {
    status_code: Option<i32>,
    message: Option<String>,
    content: Option<String>,
}

pub struct LangChain {
    url: String,
    timeout: String,
}

impl Default for LangChain {
    fn default() -> LangChain {
        return LangChain {
            url: Config::get(ConfigKey::LangChainURL),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for LangChain {
    fn name(&self) -> BackendName {
        return BackendName::LangChain;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("LangChain URL is not defined");
        }

        let res = reqwest::Client::new()
            .get(format!("{url}/openapi.json", url = self.url))
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "LangChain is not reachable");
            bail!("LangChain is not reachable");
        }

        let status = res.unwrap().status().as_u16();
        if status >= 400 {
            tracing::error!(status = status, "LangChain health check failed");
            bail!("LangChain health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let res = reqwest::Client::new()
            .get(format!("{url}/openapi.json", url = self.url))
            .send()
            .await?
            .json::<OpenAPIJSONResponse>()
            .await?;

        let mut models = res
            .paths
            .keys()
            .filter_map(|url_path| {
                if !url_path.ends_with("/stream") || url_path.contains("{config_hash}") {
                    return None;
                }

                let model = url_path.replace("/stream", "");
                return Some(model[1..model.len()].to_string());
            })
            .unique()
            .collect::<Vec<String>>();

        models.sort();

        return Ok(models);
    }

    #[allow(clippy::implicit_return)]
    async fn get_completion<'a>(
        &self,
        prompt: BackendPrompt,
        tx: &'a mpsc::UnboundedSender<Event>,
    ) -> Result<()> {
        let mut input = HashMap::new();
        // TODO consider making the key configurable.
        input.insert("question".to_string(), prompt.text);

        let req = CompletionRequest { input };

        let res = reqwest::Client::new()
            .post(format!(
                "{url}/{model}/stream",
                url = self.url,
                model = Config::get(ConfigKey::Model)
            ))
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to LangChain"
            );
            bail!("Failed to make completion request to LangChain");
        }

        let stream = res.bytes_stream().map_err(convert_err);
        let mut lines_reader = StreamReader::new(stream).lines();

        while let Ok(line) = lines_reader.next_line().await {
            if line.is_none() {
                break;
            }
            let mut cleaned_line = line.unwrap().trim().to_string();
            if !cleaned_line.starts_with("data:") {
                continue;
            }
            cleaned_line = cleaned_line.split_off(5).trim().to_string();
            let ores: CompletionResponse = serde_json::from_str(&cleaned_line).unwrap();

            if let Some(status_code) = ores.status_code {
                if status_code >= 400 {
                    return Err(anyhow!(ores.message.unwrap()));
                }
            }

            if ores.content.is_none() {
                continue;
            }
            let text = ores.content.unwrap();
            if text.is_empty() {
                continue;
            }

            let msg = BackendResponse {
                author: Author::Model,
                text,
                done: false,
                context: None,
            };
            tx.send(Event::BackendPromptResponse(msg))?;
        }

        let msg = BackendResponse {
            author: Author::Model,
            text: "".to_string(),
            done: true,
            context: Some("not-supported".to_string()),
        };
        tx.send(Event::BackendPromptResponse(msg))?;

        return Ok(());
    }
}
