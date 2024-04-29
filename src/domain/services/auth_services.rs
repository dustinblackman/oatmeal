#[cfg(test)]
#[path = "auth_services_test.rs"]
mod tests;

use crate::configuration::Config;
use crate::configuration::ConfigKey;
use std::fs;
use std::io::Read;
use std::io::Write;
use std::path::PathBuf;
use std::thread;
use std::time::Duration;

use anyhow::bail;
use anyhow::Result;
use serde::Deserialize;
use serde::Serialize;
use strum::EnumIter;
use strum::EnumVariantNames;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, EnumVariantNames, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum AuthService {
    GithubCopilot,
}

impl AuthService {
    pub fn parse(text: String) -> Option<AuthService> {
        return AuthService::iter().find(|e| return e.to_string() == text);
    }
}

pub struct AuthGithubCopilot {
    pub device_code: Option<String>,
    pub file_path: PathBuf,
    api_url: String,
    login_url: String,
}

impl Default for AuthGithubCopilot {
    fn default() -> AuthGithubCopilot {
        // let path = Config::get(ConfigKey::GithubcopilotAuthFile);
        // let file_path = fs::canonicalize(PathBuf::from(path)).unwrap();
        // println!("Path {}", file_path.to_str().unwrap());

        return AuthGithubCopilot {
            device_code: None,
            file_path: PathBuf::from(Config::get(ConfigKey::GithubcopilotAuthFile)),
            api_url: "https://api.github.com".to_string(),
            login_url: "https://github.com".to_string(),
        };
    }
}

impl AuthGithubCopilot {
    pub async fn run_auth(&mut self) -> Result<String> {
        let mut oauth_token = self.get_cached_oauth_token();
        if oauth_token.is_err() {
            let res = self.start_github_device_login().await;
            if res.is_err() {
                bail!("Failed to start device login");
            }
            let res = res.unwrap();

            let verification_uri = res.clone().verification_uri;

            println!(
                "Please go to {verification_uri} and enter the code {user_code}",
                verification_uri = verification_uri,
                user_code = res.clone().user_code,
            );

            let device_code = res.clone().device_code;
            self.device_code = Some(device_code);
            let mut token = self.check_github_token().await;
            while token.is_err() {
                thread::sleep(Duration::from_secs(5));
                let res = self.check_github_token().await;
                token = res;
            }
            oauth_token = token;
        }
        return match oauth_token {
            Ok(_) => Ok("Authorization complete".to_string()),
            Err(_) => bail!("Failed to get oauth token"),
        };
    }

    pub fn get_cached_oauth_token(&self) -> Result<String> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .open(self.file_path.clone())?;

        let mut contents = String::new();
        let _ = file.read_to_string(&mut contents);

        let json_data: HostData = serde_json::from_str(&contents)?;

        if let Some(token) = json_data.github_com.and_then(|x| return x.oauth_token) {
            return Ok(token);
        }
        bail!("Github Copilot token not found")
    }

    fn set_cached_oauth_token(&self, user_login: String, token: String) -> Result<()> {
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .truncate(true)
            .create(true)
            .open(self.file_path.clone())?;

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

    async fn start_github_device_login(&self) -> Result<GithubDeviceCodeResponse> {
        let req = GithubDeviceCodeRequest {
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            scope: "read:user".to_string(),
        };
        let res = reqwest::Client::new()
            .post(format!("{url}/login/device/code", url = self.login_url))
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

    async fn check_github_token(&self) -> Result<String> {
        let device_code = match &self.device_code {
            Some(code) => code,
            None => bail!("Device code is not set"),
        };

        let req = GithubAccessTokenRequest {
            device_code: device_code.to_string(),
            client_id: "Iv1.b507a08c87ecfe98".to_string(),
            grant_type: "urn:ietf:params:oauth:grant-type:device_code".to_string(),
        };

        let token_res = reqwest::Client::new()
            .post(format!(
                "{url}/login/oauth/access_token",
                url = self.login_url,
            ))
            .header("Accept", "application/json")
            .header("Content-Type", "application/json")
            .header("User-Agent", "GitHubCopilot/1.133.0")
            .header("editor-version", "vscode/1.85.1")
            .json(&req)
            .send()
            .await;

        let token_text = token_res?.text().await?;
        let token_result: GithubAccessTokenResponse = serde_json::from_str(&token_text)?;
        let oauth_token = token_result.access_token.clone();

        let user_res = reqwest::Client::new()
            .get(format!("{url}/user", url = self.api_url))
            .header(
                "Authorization",
                format!(
                    "{token_type} {access_token}",
                    token_type = token_result.token_type,
                    access_token = oauth_token
                ),
            )
            .header("Accept", "application/json")
            .header("User-Agent", "GitHubCopilot/1.133.0")
            .send()
            .await;
        let user_text = user_res?.text().await?;
        let user_data: GithubUser = serde_json::from_str(&user_text)?;
        let _ = self.set_cached_oauth_token(user_data.login, oauth_token);

        return Ok(token_result.access_token.clone());
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
struct GithubDeviceCodeRequest {
    client_id: String,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubDeviceCodeResponse {
    device_code: String,
    user_code: String,
    verification_uri: String,
    expires_in: u64,
    interval: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubAccessTokenRequest {
    client_id: String,
    device_code: String,
    grant_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubAccessTokenResponse {
    access_token: String,
    token_type: String,
    scope: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GithubUser {
    login: String,
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
