extern crate tempdir;

use crate::domain::services::AuthGithubCopilot;
use crate::domain::services::GithubAccessTokenRequest;
use crate::domain::services::GithubAccessTokenResponse;
use crate::domain::services::GithubDeviceCodeResponse;
use anyhow::bail;
use anyhow::Result;
use std::io::Read;
use std::io::Write;
use std::path::Path;
use std::path::PathBuf;
use tempdir::TempDir;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::GithubUser;

impl AuthGithubCopilot {
    fn with_path(file_path: PathBuf, url: String) -> AuthGithubCopilot {
        return AuthGithubCopilot {
            device_code: None,
            file_path,
            login_url: url.clone(),
            api_url: url.clone(),
        };
    }
}

#[tokio::test]
async fn it_loads_good_githubcopilot_file() -> Result<()> {
    let file_path = Path::new("./test").join("githubcopilot-host.json");
    let mut auth = AuthGithubCopilot::with_path(file_path, "".to_string());
    let res = auth.run_auth().await?;
    assert_eq!(res, "Authorization complete");
    return Ok(());
}

#[tokio::test]
async fn it_no_githubcopilot_file() -> Result<()> {
    let tmp_dir = TempDir::new("githubcopilot")?;
    let file_path = tmp_dir.path().join("hosts.json");
    let mut server = mockito::Server::new();

    let device_code_response = serde_json::to_string(&GithubDeviceCodeResponse {
        device_code: "1234".to_string(),
        user_code: "".to_string(),
        verification_uri: "".to_string(),
        interval: 5,
        expires_in: 1800,
    })?;
    let device_code_mock = server
        .mock("POST", "/login/device/code")
        .match_header("Accept", "application/json")
        .match_header("Content-Type", "application/json")
        .match_header("User-Agent", "GitHubCopilot/1.133.0")
        .match_header("editor-version", "vscode/1.85.1")
        .with_status(200)
        .with_body(device_code_response)
        .create();

    let check_reponse = serde_json::to_string(&GithubAccessTokenResponse {
        access_token: "token123".to_string(),
        token_type: "bearer".to_string(),
        scope: "repo".to_string(),
    })?;
    let confirm_mock = server
        .mock("POST", "/login/oauth/access_token")
        .match_header("Accept", "application/json")
        .match_header("Content-Type", "application/json")
        .match_header("User-Agent", "GitHubCopilot/1.133.0")
        .match_header("editor-version", "vscode/1.85.1")
        .with_status(200)
        .with_body(check_reponse)
        .create();

    let user = serde_json::to_string(&GithubUser {
        login: "test".to_string(),
    })?;
    let user_mock = server
        .mock("GET", "/user")
        .match_header("Authorization", "bearer token123")
        .match_header("Accept", "application/json")
        .match_header("User-Agent", "GitHubCopilot/1.133.0")
        .with_status(200)
        .with_body(user)
        .create();

    let mut auth = AuthGithubCopilot::with_path(file_path, server.url());
    let res = auth.run_auth().await?;

    device_code_mock.assert();
    confirm_mock.assert();
    user_mock.assert();

    assert_eq!(res, "Authorization complete");
    return Ok(());
}
