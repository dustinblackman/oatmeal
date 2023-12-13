use anyhow::Result;

use super::NoopEditor;
use crate::domain::models::AcceptType;
use crate::domain::models::Editor;
use crate::domain::models::EditorContext;

#[tokio::test]
async fn it_successfully_health_checks() -> Result<()> {
    NoopEditor::default().health_check().await?;
    return Ok(());
}

#[tokio::test]
async fn it_returns_no_context() -> Result<()> {
    let res = NoopEditor::default().get_context().await?;
    assert!(res.is_none());
    return Ok(());
}

#[tokio::test]
async fn it_clears_context() -> Result<()> {
    NoopEditor::default().get_context().await?;
    return Ok(());
}

#[tokio::test]
async fn it_returns_an_error_sending_codeblocks() -> Result<()> {
    let err = NoopEditor::default()
        .send_codeblock(EditorContext::default(), "".to_string(), AcceptType::Append)
        .await
        .unwrap_err();

    insta::assert_snapshot!(err.to_string(), @"None/noop editor does not support copying codeblocks. Consider using the 'clipboard' editor instead");
    return Ok(());
}
