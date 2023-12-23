use std::env;
use std::str;

use anyhow::bail;
use anyhow::Result;
use async_trait::async_trait;
use base64::engine::general_purpose::STANDARD as b64;
use base64::Engine;
use serde::Deserialize;
use serde::Deserializer;
use serde::Serialize;
use tokio::fs::File;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

use crate::domain::models::AcceptType;
use crate::domain::models::Editor;
use crate::domain::models::EditorContext;
use crate::domain::models::EditorName;

fn base64_to_string<'de, D: Deserializer<'de>>(deserializer: D) -> Result<String, D::Error> {
    let val = match serde::de::Deserialize::deserialize(deserializer)? {
        serde_json::Value::String(s) => s,
        _ => return Err(serde::de::Error::custom("Wrong type, expected string")),
    };

    let b64_res = b64.decode(val).unwrap();
    let str_res = str::from_utf8(&b64_res).unwrap().to_string();

    return Ok(str_res);
}

#[derive(Debug, Deserialize, Serialize)]
struct ContextResponse {
    file_path: String,
    language: String,
    #[serde(deserialize_with = "base64_to_string")]
    code: String,
    start_line: i64,
    end_line: Option<i64>,
}

#[derive(Debug, Serialize)]
struct SubmitChangesRequest {
    accept_type: String,
    file_path: String,
    code: String,
    start_line: i64,
    end_line: Option<i64>,
}

impl From<ContextResponse> for EditorContext {
    fn from(val: ContextResponse) -> Self {
        return EditorContext {
            file_path: val.file_path,
            language: val.language,
            code: val.code,
            start_line: val.start_line,
            end_line: val.end_line,
        };
    }
}

async fn run_lua_command(func: &str) -> Result<String> {
    let nvim_server_path = env::var("NVIM")?;
    let lua_func = format!("v:lua.{func}");
    let args = vec![
        "--headless",
        "--server",
        &nvim_server_path,
        "--remote-expr",
        &lua_func,
    ];

    let stdout = Command::new("nvim")
        .args(args.clone())
        .output()
        .await?
        .stdout;
    let res = String::from_utf8(stdout)?;

    tracing::error!(args = ?args, res = ?res, "Neovim requeest/response");

    return Ok(res);
}

#[derive(Default)]
pub struct Neovim {}

#[async_trait]
impl Editor for Neovim {
    fn name(&self) -> EditorName {
        return EditorName::Neovim;
    }

    #[allow(clippy::implicit_return)]
    async fn health_check(&self) -> Result<()> {
        if env::var("NVIM").is_err() {
            bail!("Not running within a Neovim terminal")
        }

        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn get_context(&self) -> Result<Option<EditorContext>> {
        let json_str = run_lua_command("oatmeal_get_context()").await?;
        if json_str.trim() == "[]" {
            return Ok(None);
        }
        let ctx: ContextResponse = serde_json::from_str(&json_str)?;

        return Ok(Some(ctx.into()));
    }

    #[allow(clippy::implicit_return)]
    async fn clear_context(&self) -> Result<()> {
        run_lua_command("oatmeal_clear_context()").await?;
        return Ok(());
    }

    #[allow(clippy::implicit_return)]
    async fn send_codeblock<'a>(
        &self,
        context: EditorContext,
        codeblock: String,
        accept_type: AcceptType,
    ) -> Result<()> {
        let req = SubmitChangesRequest {
            accept_type: accept_type.to_string(),
            file_path: context.file_path,
            code: codeblock,
            start_line: context.start_line,
            end_line: context.end_line,
        };

        let json_str = serde_json::to_string(&req)?;

        let temp_file_path = env::temp_dir().join("oatmeal-context");
        let mut file = File::create(&temp_file_path).await?;
        file.write_all(json_str.as_bytes()).await?;
        file.sync_all().await?;

        run_lua_command(&format!(
            "oatmeal_submit_changes(\"{}\")",
            temp_file_path.display()
        ))
        .await?;

        return Ok(());
    }
}
