use anyhow::bail;
use anyhow::Result;
use rand::Rng;
use serde::Deserialize;
use serde::Serialize;
use std::io::Read;
use std::io::Write;
use std::iter;
use std::path::Path;
use std::thread;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::mpsc;

use crate::domain::models::Author;
use crate::domain::models::Event;
use crate::domain::models::Message;

fn generate_hex_string(length: usize) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
    let mut rng = rand::thread_rng();
    let one_char = || CHARSET[rng.gen_range(0..CHARSET.len())] as char;
    iter::repeat_with(one_char).take(length).collect()
}

pub struct GithubAuth {
    pub token: String,
    pub expires_at: u64,
    pub machine_id: String,
    pub vscode_sessionid: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubDeviceCodeRequest {
    client_id: String,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubAccessTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubAccessTokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
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

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct CopilotTokenResponse {
    token: String,
    expires_at: u64,
    chat_enabled: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubUser {
    login: String,
}

impl GithubAuth {
    pub async fn new<'a>(tx: &'a mpsc::UnboundedSender<Event>) -> Result<GithubAuth> {
        let mut oauth_token = Self::get_cached_oauth_token();
        if oauth_token.is_err() {
            let res = GithubAuth::start_github_device_login().await;
            if res.is_err() {
                bail!("Failed to start device login");
            }
            let res = res.unwrap();

            let verification_uri = res.clone().verification_uri;
            let text = format!(
                "
Please go to [{verification_uri}] and enter the code {user_code}
```json
{{ 
     \"url\": \"{verification_uri}\",
     \"code\": \"{user_code}\"
}}
```",
                verification_uri = verification_uri,
                user_code = res.clone().user_code,
            );

            let msg = Message::new(Author::Oatmeal, &text);
            tx.send(Event::BackendMessage(msg))?;

            let device_code = res.clone().device_code;
            let mut token = GithubAuth::check_github_token(&device_code).await;
            while token.is_err() {
                thread::sleep(Duration::from_secs(5));
                let res = GithubAuth::check_github_token(&device_code).await;
                token = res;
            }
            oauth_token = token;
        }
        let auth = GithubAuth::get_copilot_token(oauth_token.unwrap()).await;
        if auth.is_err() {
            bail!("Failed to get copilot token");
        }

        let auth = auth.unwrap();

        let time = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis()
            .to_string();
        let vscode_sessionid = uuid::Uuid::new_v4().to_string() + &time;
        let machine_id = generate_hex_string(65);

        Ok(GithubAuth {
            token: auth.token,
            expires_at: auth.expires_at,
            machine_id,
            vscode_sessionid,
        })
    }

    async fn get_copilot_token(oauth_token: String) -> Result<CopilotTokenResponse> {
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
            } else {
                bail!("Github Copilot token not found");
            }
        } else {
            bail!("Github Copilot token not found");
        }
    }

    async fn start_github_device_login() -> Result<GithubDeviceCodeResponse> {
        let req = GithubDeviceCodeRequest {
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            scope: "read:user".to_string(),
        };
        let res = reqwest::Client::new()
            .post("https://github.com/login/device/code".to_string())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "GitHubCopilot/1.133.0")
            .header("editor-version", "vscode/1.85.1")
            .json(&req)
            .send()
            .await;

        let text = res?.text().await?;
        let token_result: GithubDeviceCodeResponse = serde_json::from_str(&text)?;
        return Ok(token_result);
    }

    async fn check_github_token(device_code: &String) -> Result<String> {
        let req = GithubAccessTokenRequest {
            device_code: device_code.clone(),
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };

        let token_res = reqwest::Client::new()
            .post("https://github.com/login/oauth/access_token".to_string())
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "GitHubCopilot/1.133.0")
            .header("editor-version", "vscode/1.85.1")
            .json(&req)
            .send()
            .await;

        let token_text = token_res?.text().await?;
        let token_result: GithubAccessTokenResponse = serde_json::from_str(&token_text)?;
        let access_token = token_result.access_token.clone();

        let user_res = reqwest::Client::new()
            .get("https://api.github.com/user".to_string())
            .header(
                "Authorization",
                format!(
                    "{token_type} {access_token}",
                    token_type = token_result.token_type,
                    access_token = access_token
                ),
            )
            .header("Accept", "application/json")
            .header("User-Agent", "GitHubCopilot/1.133.0")
            .send()
            .await;
        let user_text = user_res?.text().await?;
        let user_data: GithubUser = serde_json::from_str(&user_text)?;
        let _ = GithubAuth::set_cached_oauth_token(user_data.login, access_token);

        return Ok(token_result.access_token.clone());
    }

    fn set_cached_oauth_token(user_login: String, token: String) -> Result<()> {
        let home_dir = std::env::var("HOME").expect("HOME environment variable is not set");
        let file_path = Path::new(&home_dir).join(".config/github-copilot/hosts.json");
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(file_path.clone())?;

        let mut contents = String::new();
        let read_content = file.read_to_string(&mut contents);
        if read_content.is_err() {
            contents = "{ \"github.com\": {} }".to_string();
        }

        let user = Some(user_login.clone());
        let oauth_token = Some(token.clone());

        let mut json_data: HostData = match serde_json::from_str(&contents) {
            Ok(data) => data,
            Err(_) => HostData { github_com: None },
        };

        if json_data.github_com.is_none() {
            json_data.github_com = Some(GithubCom { user, oauth_token });
        } else {
            let mut github_com = json_data.clone().github_com.unwrap();
            if github_com.user.is_none() {
                github_com.user = user;
            }
            github_com.oauth_token = oauth_token;
            json_data.github_com = Some(github_com);
        }

        let text = serde_json::to_string(&json_data)?;
        let w = file.write_all(text.as_bytes());
        if w.is_err() {
            bail!(
                "Something went wrong! {err}",
                err = w.err().unwrap().to_string()
            );
        }
        file.flush()?;

        return Ok(());
    }
}
