#[cfg(test)]
#[path = "noop_test.rs"]
mod tests;

use anyhow::anyhow;
use anyhow::Result;

use crate::domain::models::AcceptType;
use crate::domain::models::Editor;
use crate::domain::models::EditorContext;
use crate::domain::models::EditorName;

#[derive(Default)]
pub struct NoopEditor {}

impl Editor for NoopEditor {
    fn name(&self) -> EditorName {
        return EditorName::None;
    }

    async fn health_check(&self) -> Result<()> {
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
        _codeblock: String,
        _accept_type: AcceptType,
    ) -> Result<()> {
        return Err(anyhow!(
            "None/noop editor does not support copying codeblocks. Consider using the 'clipboard' editor instead"
        ));
    }
}
