use anyhow::bail;
use anyhow::Result;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::Config;
use super::Content;
use super::ContentParts;
use super::Gemini;
use super::Model;
use super::ModelListResponse;
use crate::configuration::ConfigKey;
use crate::domain::models::Author;
use crate::domain::models::Backend;
use crate::domain::models::BackendPrompt;
use crate::domain::models::BackendResponse;
use crate::domain::models::Event;

impl Gemini {
    fn with_url(url: String) -> Gemini {
        return Gemini {
            url,
            token: "abc".to_string(),
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
    Config::set(ConfigKey::Model, "model-1");
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/v1beta/model-1?key=abc")
        .with_status(200)
        .create();

    let backend = Gemini::with_url(server.url());
    let res = backend.health_check().await;

    assert!(res.is_ok());
    mock.assert();
}

#[tokio::test]
async fn it_successfully_health_checks_with_official_api() {
    Config::set(ConfigKey::Model, "models/gemini-pro");
    let token = match std::env::var("OATMEAL_GEMINI_TOKEN") {
        Ok(token) => token,
        Err(_) => {
            println!("There is no token in environment defined, skipping test");
            return;
        }
    };
    let backend = Gemini {
        url: "https://generativelanguage.googleapis.com".to_string(),
        token,
        timeout: "500".to_string(),
    };

    let res = backend.health_check().await;
    assert!(res.is_ok());
}

#[tokio::test]
async fn it_fails_health_checks() {
    Config::set(ConfigKey::Model, "model-1");
    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/v1beta/model-1?key=abc")
        .with_status(500)
        .create();

    let backend = Gemini::with_url(server.url());
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
                supported_generation_methods: vec!["generateContent".to_string()],
            },
            Model {
                name: "second".to_string(),
                supported_generation_methods: vec!["generateContent".to_string()],
            },
        ],
    })?;

    let mut server = mockito::Server::new();
    let mock = server
        .mock("GET", "/v1beta/models?key=abc")
        .with_status(200)
        .with_body(body)
        .create();

    let backend = Gemini::with_url(server.url());
    let res = backend.list_models().await?;
    mock.assert();

    assert_eq!(res, vec!["first".to_string(), "second".to_string()]);

    return Ok(());
}

#[tokio::test]
async fn it_gets_completions() -> Result<()> {
    Config::set(ConfigKey::Model, "model-1");
    let body = [
        "[",
        "\"contents\": [{",
        "\"parts\": [{",
        "\"text\": \"Hello \"",
        "}]",
        "},",
        "{",
        "\"parts\": [{",
        "\"text\": \"World\"",
        "}]",
        "},",
        "{",
        "\"parts\": [{",
        "\"text\": \"\"",
        "}]",
        "}]",
        "]",
    ]
    .join("\n");
    let prompt = BackendPrompt {
        text: "Say hi to the world".to_string(),
        backend_context: serde_json::to_string(&vec![Content {
            role: "model".to_string(),
            parts: vec![ContentParts::Text("Hello".to_string())],
        }])?,
    };

    let mut server = mockito::Server::new();
    let mock = server
        .mock("POST", "/v1beta/model-1:streamGenerateContent?key=abc")
        .with_status(200)
        .with_body(body)
        .create();

    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();

    let backend = Gemini::with_url(server.url());
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
    assert_eq!(third_recv.text, "".to_string());
    assert!(third_recv.done);
    insta_snapshot(|| {
        insta::assert_toml_snapshot!(third_recv.context);
    });

    return Ok(());
}
