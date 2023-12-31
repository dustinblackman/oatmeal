use anyhow::anyhow;
use anyhow::Result;

use crate::domain::models::AcceptType;
use crate::domain::models::Editor;
use crate::domain::models::EditorContext;
use crate::domain::models::EditorName;
use crate::domain::services::clipboard::ClipboardService;

#[derive(Default)]
pub struct Clipboard {}

impl Editor for Clipboard {
    fn name(&self) -> EditorName {
        return EditorName::Clipboard;
    }

    async fn health_check(&self) -> Result<()> {
        if let Err(err) = ClipboardService::healthcheck() {
            return Err(anyhow! {format!("Clipboard editor failed to initialize: {err}")});
        }

        return Ok(());
    }

    async fn get_context(&self) -> Result<Option<EditorContext>> {
        return Ok(None);
    }

    async fn clear_context(&self) -> Result<()> {
        return Ok(());
    }

    async fn send_codeblock<'a>(
        &self,
        _context: EditorContext,
        codeblock: String,
        _accept_type: AcceptType,
    ) -> Result<()> {
        ClipboardService::set(codeblock)?;
        return Ok(());
    }
}
