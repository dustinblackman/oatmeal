use anyhow::bail;
use anyhow::Result;
use mockito::Matcher;
use mockito::Mock;
use mockito::ServerGuard;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::CompletionChoiceResponse;
use super::CompletionDeltaResponse;
use super::CompletionResponse;
use super::CopilotTokenResponse;
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
            auth_url: "".to_string(),
            timeout: "200".to_string(),
            machine_id: "def".to_string(),
            vscode_sessionid: "ghi".to_string(),
            oauth_token: Some("abc".to_string()),
        };
    }
    fn with_data(
        url: String,
        machine_id: String,
        vscode_sessionid: String,
        oauth_token: String,
    ) -> GithubCopilot {
        return GithubCopilot {
            url: url.clone(),
            auth_url: url.clone(),
            timeout: "200".to_string(),
            machine_id,
            vscode_sessionid,
            oauth_token: Some(oauth_token),
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
async fn it_gets_completions_no_oauth() -> Result<()> {
    let MockGithubCopilot { backend, .. } = setup_get_completion("".to_string(), "".to_string())?;
    let (tx, _) = mpsc::unbounded_channel::<Event>();
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![MessageRequest {
            role: "system".to_string(),
            content: "How may I help you?".to_string(),
        }])?,
    };
    let res = backend.get_completion(prompt, &tx).await;
    assert!(res.is_err());
    return Ok(());
}

#[tokio::test]
async fn it_gets_completions_no_token() -> Result<()> {
    let new_token = "no_current_token".to_string();
    let MockGithubCopilot {
        comp_mock,
        _server,
        auth_mock,
        backend,
    } = setup_get_completion("oauth-12345".to_string(), new_token.clone())?;

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![MessageRequest {
            role: "system".to_string(),
            content: "How may I help you?".to_string(),
        }])?,
    };
    backend.get_completion(prompt, &tx).await?;

    comp_mock.assert();
    auth_mock.assert();

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

    let context = third_recv.clone().context.expect("Context must be defined");

    let context_messages: Vec<MessageRequest> = serde_json::from_str(&context)?;
    let index = context_messages
        .iter()
        .position(|value| return value.role == *"__token")
        .expect("Token must be set");

    let msg = context_messages.get(index).expect("Token must be set");
    let content = serde_json::from_str::<CopilotTokenResponse>(&msg.content)?;
    assert_eq!(content.token, new_token);
    assert!(content.expires_at > 0);

    insta_snapshot(|| {
        insta::assert_toml_snapshot!(third_recv.context);
    });

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions_token_expired() -> Result<()> {
    let new_token = "token_expired".to_string();
    let current_expires_at: u64 = 1577907680; // Expired in 2020
    let MockGithubCopilot {
        comp_mock,
        _server,
        auth_mock,
        backend,
    } = setup_get_completion("oauth-12345".to_string(), new_token.clone())?;

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![
            MessageRequest {
                role: "system".to_string(),
                content: "How may I help you?".to_string(),
            },
            MessageRequest {
                role: "__token".to_string(),
                content: serde_json::to_string(&CopilotTokenResponse {
                    token: "abc".to_string(),       // Old token
                    expires_at: current_expires_at, // Expired in 2020
                    chat_enabled: true,
                })?,
            },
        ])?,
    };
    backend.get_completion(prompt, &tx).await?;

    comp_mock.assert();
    auth_mock.assert();

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

    let context = third_recv.clone().context.expect("Context must be defined");

    let context_messages: Vec<MessageRequest> = serde_json::from_str(&context)?;
    let index = context_messages
        .iter()
        .position(|value| return value.role == *"__token")
        .expect("Token must be set");

    let msg = context_messages.get(index).expect("Token must be set");
    let content = serde_json::from_str::<CopilotTokenResponse>(&msg.content)?;
    assert_eq!(content.token, new_token);
    assert!(content.expires_at > current_expires_at);

    insta_snapshot(|| {
        insta::assert_toml_snapshot!(third_recv.context);
    });

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    let token = "token_ok".to_string();
    let MockGithubCopilot {
        comp_mock,
        _server,
        auth_mock,
        backend,
    } = setup_get_completion("oauth-12345".to_string(), token.clone())?;
    // In this test case expiration does not matter, endpoint must not be reached.

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![
            MessageRequest {
                role: "system".to_string(),
                content: "How may I help you?".to_string(),
            },
            MessageRequest {
                role: "__token".to_string(),
                content: serde_json::to_string(&CopilotTokenResponse {
                    token,
                    expires_at: 2025629947, // long long time
                    chat_enabled: true,
                })?,
            },
        ])?,
    };
    backend.get_completion(prompt, &tx).await?;

    comp_mock.assert();
    auth_mock.expect(0).assert();

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

struct MockGithubCopilot {
    _server: mockito::ServerGuard,
    auth_mock: Mock,
    comp_mock: Mock,
    backend: GithubCopilot,
}

fn setup_get_completion(oauth: String, token: String) -> Result<MockGithubCopilot> {
    let machine_id = "machineid-12345";
    let vscode_sessionid = "vscode-12345";
    let expires_at: u64 = 2025629947;

    let auth_response = serde_json::to_string(&CopilotTokenResponse {
        token: token.to_string(),
        expires_at,
        chat_enabled: true,
    })?;

    let mut auth_server = mockito::Server::new();
    let auth_mock = auth_server
        .mock("GET", "/copilot_internal/v2/token")
        .match_header(
            "Authorization",
            Matcher::Exact(format!("token {oauth}", oauth = oauth).to_string()),
        )
        .match_header("editor-version", "vscode/1.85.1")
        .match_header("editor-plugin-version", "copilot-chat/0.4.1")
        .match_header("user-agent", "GitHubCopilotChat/0.4.1")
        .with_status(200)
        .with_body(auth_response)
        .create();

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

    // let mut comp_server: ServerGuard = mockito::Server::new();
    let comp_mock = auth_server
        .mock("POST", "/chat/completions")
        .match_header(
            "Authorization",
            Matcher::Exact(format!("Bearer {token}", token = token)),
        )
        .match_header("vscode-sessionid", vscode_sessionid)
        .match_header("machine-id", machine_id)
        .match_header("content-type", "application/json")
        .match_header("user-agent", "GitHubCopilotChat/0.4.1")
        .match_header("editor-version", "vscode/1.85.1")
        .match_header("editor-plugin-version", "copilot-chat/0.4.1")
        .match_header("openai-organization", "github-copilot")
        .match_header("openai-intent", "conversation-panel")
        .with_status(200)
        .with_body(body)
        .create();

    let backend = GithubCopilot::with_data(
        auth_server.url(),
        machine_id.to_string(),
        vscode_sessionid.to_string(),
        oauth.to_string(),
    );

    return Ok(MockGithubCopilot {
        _server: auth_server,
        auth_mock,
        comp_mock,
        backend,
    });
}
