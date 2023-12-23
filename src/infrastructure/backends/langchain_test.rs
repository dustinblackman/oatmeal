use std::collections::HashMap;

use anyhow::bail;
use anyhow::Result;
use tokio::sync::mpsc;

use super::CompletionResponse;
use super::LangChain;
use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;
use crate::infrastructure::backends::langchain::Empty;
use crate::infrastructure::backends::langchain::OpenAPIJSONResponse;

impl LangChain {
    fn with_url(url: String) -> LangChain {
        return LangChain {
            url,
            timeout: "200".to_string(),
        };
    }
}

fn to_res(action: Option<Event>) -> Result<BackendResponse> {
    let act = match action.unwrap() {
        Event::BackendPromptResponse(res) => res,
        _ => bail!("Wrong type from recv"),
    };

    return Ok(act);
}

#[tokio::test]
async fn it_successfully_health_checks() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/openapi.json")
        .with_status(200)
        .create();

    let backend = LangChain::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_ok());
    mock.assert();
}

#[tokio::test]
async fn it_fails_health_checks() {
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/openapi.json")
        .with_status(500)
        .create();

    let backend = LangChain::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_err());
    mock.assert();
}

#[tokio::test]
async fn it_lists_models() -> Result<()> {
    let mut paths = HashMap::new();
    paths.insert("/model-1/stream".to_string(), Empty {});
    paths.insert("/model-2/stream".to_string(), Empty {});
    paths.insert("/model-2/{config_hash}/stream".to_string(), Empty {});
    paths.insert("/other".to_string(), Empty {});
    let body = serde_json::to_string(&OpenAPIJSONResponse { paths })?;

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/openapi.json")
        .with_status(200)
        .with_body(body)
        .create();

    let backend = LangChain::with_url(server.url());
    let res = backend.list_models().await?;
    mock.assert();

    assert_eq!(res, vec!["model-1".to_string(), "model-2".to_string()]);

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    Config::set(ConfigKey::Model, "model-1");

    let first_line = serde_json::to_string(&CompletionResponse {
        status_code: None,
        message: None,
        content: Some("Hello ".to_string()),
    })?;

    let second_line = serde_json::to_string(&CompletionResponse {
        status_code: None,
        message: None,
        content: Some("World".to_string()),
    })?;

    let body = [
        "event: garbadge",
        "",
        &format!("data: {first_line}"),
        "event: garbadge",
        &format!("data: {second_line}"),
        "",
    ]
    .join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: "".to_string(),
    };

    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/model-1/stream")
        .with_status(200)
        .with_body(body)
        .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = LangChain::with_url(server.url());
    backend.get_completion(prompt, &tx).await?;

    mock.assert();

    let first_recv = to_res(rx.recv().await)?;
    let second_recv = to_res(rx.recv().await)?;
    let third_recv = to_res(rx.recv().await)?;

    assert_eq!(first_recv.author, Author::Model);
    assert_eq!(first_recv.text, "Hello ".to_string());
    assert!(!first_recv.done);
    assert_eq!(first_recv.context, None);

    assert_eq!(second_recv.author, Author::Model);
    assert_eq!(second_recv.text, "World".to_string());
    assert!(!second_recv.done);
    assert_eq!(second_recv.context, None);

    assert_eq!(third_recv.author, Author::Model);
    assert!(third_recv.text.is_empty());
    assert!(third_recv.done);
    assert_eq!(third_recv.context, Some("not-supported".to_string()));

    return Ok(());
}
