use anyhow::bail;
use anyhow::Result;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::Claude;
use super::CompletionDeltaResponse;
use super::CompletionResponse;
use super::Healthcheck;
use super::MessageRequest;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

impl Claude {
    fn with_url(url: String) -> Claude {
        return Claude {
            url,
            token: "abc".to_string(),
            timeout: "500".to_string(),
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
    let body = serde_json::to_string(&Healthcheck {
        message: "ok".to_string(),
    })
    .unwrap();

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/healthcheck")
        .with_status(200)
        .with_body(body)
        .create();

    let backend = Claude::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_ok());
    mock.assert();
}

#[tokio::test]
async fn it_successfully_health_checks_with_official_api() {
    let backend = Claude::with_url("https://api.anthropic.com".to_string());
    let res = backend.health_check().await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn it_fails_health_checks() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/healthcheck").with_status(500).create();

    let backend = Claude::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_err());
    mock.assert();
}

#[tokio::test]
async fn it_lists_models() -> Result<()> {
    let backend = Claude::with_url("https://api.anthropic.com".to_string());
    let res = backend.list_models().await?;

    assert!(!res.is_empty());
    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    let first_line = serde_json::to_string(&CompletionResponse {
        _type: "content_block_delta".to_string(),
        delta: CompletionDeltaResponse {
            _type: "text".to_string(),
            text: "Hello ".to_string(),
        },
    })?;

    let second_line = serde_json::to_string(&CompletionResponse {
        _type: "content_block_delta".to_string(),
        delta: CompletionDeltaResponse {
            _type: "text".to_string(),
            text: "World".to_string(),
        },
    })?;

    let third_line = serde_json::to_string(&CompletionResponse {
        _type: "content_block_stop".to_string(),
        delta: CompletionDeltaResponse {
            _type: "end".to_string(),
            text: "".to_string(),
        },
    })?;

    let body = [first_line, second_line, third_line].join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![MessageRequest {
            role: "assistant".to_string(),
            content: "How may I help you?".to_string(),
        }])?,
    };

    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/v1/messages")
        .match_header("x-api-key", "abc")
        .match_header("content-type", "application/json")
        .match_header("anthropic-version", "2023-06-01")
        .match_header("anthropic-beta", "messages-2023-12-15")
        .with_status(200)
        .with_body(body)
        .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = Claude::with_url(server.url());
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
    insta_snapshot(|| {
        insta::assert_toml_snapshot!(third_recv.context);
    });

    return Ok(());
}
