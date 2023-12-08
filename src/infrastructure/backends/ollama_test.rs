use anyhow::bail;
use anyhow::Result;
use tokio::sync::mpsc;

use super::CompletionResponse;
use super::Model;
use super::ModelListResponse;
use super::Ollama;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

impl Ollama {
    fn with_url(url: String) -> Ollama {
        return Ollama {
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
    let mock = server.mock("GET", "/").with_status(200).create();

    let backend = Ollama::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_ok());
    mock.assert();
}

#[tokio::test]
async fn it_fails_health_checks() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/").with_status(500).create();

    let backend = Ollama::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_err());
    mock.assert();
}

#[tokio::test]
async fn it_lists_models() -> Result<()> {
    let body = serde_json::to_string(&ModelListResponse {
        models: vec![
            Model {
                name: "first".to_string(),
            },
            Model {
                name: "second".to_string(),
            },
        ],
    })?;

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/api/tags")
        .with_status(200)
        .with_body(body)
        .create();

    let backend = Ollama::with_url(server.url());
    let res = backend.list_models().await?;

    assert_eq!(res, vec!["first".to_string(), "second".to_string()]);
    mock.assert();

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    let first_line = serde_json::to_string(&CompletionResponse {
        response: "Hello ".to_string(),
        done: false,
        context: None,
    })?;

    let second_line = serde_json::to_string(&CompletionResponse {
        response: "World".to_string(),
        done: true,
        context: Some(vec![1, 2, 3]),
    })?;

    let body = [first_line, second_line].join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![1])?,
    };

    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/api/generate")
        .with_status(200)
        .with_body(body)
        .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = Ollama::with_url(server.url());
    backend.get_completion(prompt, &tx).await?;

    mock.assert();

    let first_recv = to_res(rx.recv().await)?;
    let second_recv = to_res(rx.recv().await)?;

    assert_eq!(first_recv.author, Author::Model);
    assert_eq!(first_recv.text, "Hello ".to_string());
    assert!(!first_recv.done);
    assert_eq!(first_recv.context, None);

    assert_eq!(second_recv.author, Author::Model);
    assert_eq!(second_recv.text, "World".to_string());
    assert!(second_recv.done);
    assert_eq!(second_recv.context, Some("[1,2,3]".to_string()));

    return Ok(());
}
