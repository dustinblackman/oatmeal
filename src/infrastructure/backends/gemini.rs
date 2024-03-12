#[cfg(test)]
#[path = "gemini_test.rs"]
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
#[serde(rename_all = "camelCase")]
struct Model {
    name: String,
    supported_generation_methods: Vec<String>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct ModelListResponse {
    models: Vec<Model>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ContentPartsBlob {
    mime_type: String,
    data: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
enum ContentParts {
    Text(String),
    InlineData(ContentPartsBlob),
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct Content {
    role: String,
    parts: Vec<ContentParts>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CompletionRequest {
    contents: Vec<Content>,
}

#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct GenerateContentResponse {
    text: String,
}

pub struct Gemini {
    url: String,
    token: String,
    timeout: String,
}

impl Default for Gemini {
    fn default() -> Gemini {
        return Gemini {
            url: "https://generativelanguage.googleapis.com".to_string(),
            token: Config::get(ConfigKey::GeminiToken),
            timeout: Config::get(ConfigKey::BackendHealthCheckTimeout),
        };
    }
}

#[async_trait]
impl Backend for Gemini {
    fn name(&self) -> BackendName {
        return BackendName::Gemini;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if self.url.is_empty() {
            bail!("Gemini URL is not defined");
        }
        if self.token.is_empty() {
            bail!("Gemini token is not defined");
        }

        let url = format!(
            "{url}/v1beta/{model}?key={key}",
            url = self.url,
            model = Config::get(ConfigKey::Model),
            key = self.token
        );

        let res = reqwest::Client::new()
            .get(&url)
            .timeout(Duration::from_millis(self.timeout.parse::<u64>()?))
            .send()
            .await;

        if res.is_err() {
            tracing::error!(error = ?res.unwrap_err(), "Gemini is not reachable");
            bail!("Gemini is not reachable");
        }

        let status = res.unwrap().status().as_u16();
        if status >= 400 {
            tracing::error!(status = status, "Gemini health check failed");
            bail!("Gemini health check failed");
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn list_models(&self) -> Result<Vec<String>> {
        let res = reqwest::Client::new()
            .get(format!(
                "{url}/v1beta/models?key={key}",
                url = self.url,
                key = self.token
            ))
            .send()
            .await?
            .json::<ModelListResponse>()
            .await?;

        let mut models: Vec<String> = res
            .models
            .iter()
            .filter(|model| {
                model
                    .supported_generation_methods
                    .contains(&"generateContent".to_string())
            })
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
        let mut contents: Vec<Content> = vec![];
        if !prompt.backend_context.is_empty() {
            contents = serde_json::from_str(&prompt.backend_context)?;
        }
        contents.push(Content {
            role: "user".to_string(),
            parts: vec![ContentParts::Text(prompt.text)],
        });

        let req = CompletionRequest {
            contents: contents.clone(),
        };

        let res = reqwest::Client::new()
            .post(format!(
                "{url}/v1beta/{model}:streamGenerateContent?key={key}",
                url = self.url,
                model = Config::get(ConfigKey::Model),
                key = self.token,
            ))
            .json(&req)
            .send()
            .await?;

        if !res.status().is_success() {
            tracing::error!(
                status = res.status().as_u16(),
                "Failed to make completion request to Gemini"
            );
            bail!(format!(
                "Failed to make completion request to Gemini, {}",
                res.status().as_u16()
            ));
        }
        let stream = res.bytes_stream().map_err(convert_err);
        let mut lines_reader = StreamReader::new(stream).lines();

        let mut last_message = "".to_string();
        while let Ok(line) = lines_reader.next_line().await {
            if line.is_none() {
                break;
            }

            let cleaned_line = line.unwrap().trim().to_string();
            if !cleaned_line.starts_with("\"text\":") {
                continue;
            }

            let ores: GenerateContentResponse =
                serde_json::from_str(&format!("{{ {text} }}", text = cleaned_line)).unwrap();

            if ores.text.is_empty() || ores.text.is_empty() || ores.text == "\n" {
                break;
            }

            last_message += &ores.text;
            let msg = BackendResponse {
                author: Author::Model,
                text: ores.text,
                done: false,
                context: None,
            };
            tx.send(Event::BackendPromptResponse(msg))?;
        }

        contents.push(Content {
            role: "model".to_string(),
            parts: vec![ContentParts::Text(last_message.clone())],
        });

        let msg = BackendResponse {
            author: Author::Model,
            text: "".to_string(),
            done: true,
            context: Some(serde_json::to_string(&contents)?),
        };
        tx.send(Event::BackendPromptResponse(msg))?;

        return Ok(());
    }
}
