#[cfg(test)]
#[path = "editor_test.rs"]
mod tests;

use std::fmt;

use anyhow::Result;
use async_trait::async_trait;
use strum::EnumIter;
use strum::EnumVariantNames;
use strum::IntoEnumIterator;

#[derive(Clone, Debug, PartialEq, Eq, EnumIter, EnumVariantNames, strum::Display)]
#[strum(serialize_all = "lowercase")]
pub enum EditorName {
    Neovim,
    Clipboard,
    None,
}

impl EditorName {
    pub fn parse(text: String) -> Option<EditorName> {
        return EditorName::iter().find(|e| return e.to_string() == text);
    }
}

#[derive(Debug, PartialEq)]
pub enum AcceptType {
    /// Append in editor where the cursor was last.
    Append,
    /// Replace selected code in editor.
    Replace,
}

impl fmt::Display for AcceptType {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AcceptType::Append => return write!(f, "append"),
            AcceptType::Replace => return write!(f, "replace"),
        }
    }
}

#[derive(Clone, Default)]
pub struct EditorContext {
    pub file_path: String,
    pub language: String,
    pub code: String,
    pub start_line: i64,
    pub end_line: Option<i64>,
}

impl EditorContext {
    pub fn format(&self) -> String {
        let file_path = &self.file_path;
        let language = &self.language;
        let code = &self.code;

        if code.is_empty() || self.end_line.is_none() {
            return format!("File: {file_path}");
        }

        return format!(
            r#"
File: {file_path}

```{language}
{code}
```
    "#
        )
        .trim()
        .to_string();
    }
}

#[async_trait]
pub trait Editor {
    /// Returns the name of the editor.
    fn name(&self) -> EditorName;

    /// Used at startup to verify all configurations are available to work with
    /// the editor.
    async fn health_check(&self) -> Result<()>;

    /// Used to provide context from the editor such as coding language, file,
    /// selected lines, and full code blocks.
    async fn get_context(&self) -> Result<Option<EditorContext>>;

    /// If required, clear_context is called when Oatmeal exits to do any
    /// necessary cleanup in the editor.
    async fn clear_context(&self) -> Result<()>;

    /// Sends accepted code blocks back to the editor, with `accept_type`
    /// defining if code blocks should replace existing code, or append to
    /// it.
    async fn send_codeblock<'a>(
        &self,
        context: EditorContext,
        codeblock: String,
        accept_type: AcceptType,
    ) -> Result<()>;

    /// Opens the prompt in the editor.
    /// Default implementation:
    ///     - Uses the $EDITOR environment variable to get the executable and launches that in a
    ///     new process.
    ///     - Blocks the thread.
    #[allow(clippy::implicit_return)]
    async fn edit_prompt(&self, temp_file_path: &std::path::Path) -> Result<()> {
        let editor = std::env::var("EDITOR")?;
        let _status = std::process::Command::new(editor)
            .arg(temp_file_path)
            // Blocking method
            .status()?;
        return Ok(());
    }
}

pub type EditorBox = Box<dyn Editor + Send + Sync>;
