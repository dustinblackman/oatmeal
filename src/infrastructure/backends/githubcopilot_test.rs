use anyhow::bail;
use anyhow::Result;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::CompletionChoiceResponse;
use super::CompletionDeltaResponse;
use super::CompletionResponse;
use super::GithubCopilot;
use super::MessageRequest;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

impl GithubCopilot {
    fn with_url(url: String) -> GithubCopilot {
        return GithubCopilot {
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

    let backend = GithubCopilot::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_ok());
    mock.assert();
}

#[tokio::test]
async fn it_successfully_health_checks_with_official_api() {
    let backend = GithubCopilot::with_url("https://api.githubcopilot.com".to_string());
    let res = backend.health_check().await;

    assert!(res.is_ok());
}

#[tokio::test]
async fn it_fails_health_checks() {
    let mut server = mockito::Server::new();
    let mock = server.mock("GET", "/").with_status(500).create();

    let backend = GithubCopilot::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_err());
    mock.assert();
}

#[tokio::test]
async fn it_lists_models() -> Result<()> {
    let backend = GithubCopilot::default();
    let res = backend.list_models().await?;

    assert_eq!(res, vec!["gpt-4".to_string()]);

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    let first_line = serde_json::to_string(&CompletionResponse {
        choices: vec![CompletionChoiceResponse {
            delta: CompletionDeltaResponse {
                content: Some("Hello ".to_string()),
            },
            finish_reason: None,
        }],
    })?;

    let second_line = serde_json::to_string(&CompletionResponse {
        choices: vec![CompletionChoiceResponse {
            delta: CompletionDeltaResponse {
                content: Some("World".to_string()),
            },
            finish_reason: None,
        }],
    })?;

    let third_line = serde_json::to_string(&CompletionResponse {
        choices: vec![CompletionChoiceResponse {
            delta: CompletionDeltaResponse { content: None },
            finish_reason: Some("stop".to_string()),
        }],
    })?;

    let body = [first_line, second_line, third_line].join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![MessageRequest {
            role: "system".to_string(),
            content: "How may I help you?".to_string(),
        }])?,
    };

    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/chat/completions")
        .match_header("Authorization", "Bearer abc")
        // .header(
        //     "Authorization",
        //     format!("Bearer {token}", token = auth.token),
        // )
        .match_header("content-type", "application/json")
        // .header("x-request-id", uuid::Uuid::new_v4().to_string())
        // .header("vscode-sessionid", auth.vscode_sessionid)
        // .header("machine-id", auth.machine_id)
        .match_header("user-agent", "GitHubCopilotChat/0.4.1")
        .match_header("editor-version", "vscode/1.85.1")
        .match_header("editor-plugin-version", "copilot-chat/0.4.1")
        .match_header("openai-organization", "github-copilot")
        .match_header("openai-intent", "conversation-panel")
        .with_status(200)
        .with_body(body)
        .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = GithubCopilot::with_url(server.url());
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
