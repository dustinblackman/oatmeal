pub mod clipboard;
pub mod neovim;

use anyhow::bail;
use anyhow::Result;

use crate::domain::models::Editor;

pub type EditorBox = Box<dyn Editor + Send + Sync>;

pub struct EditorManager {}

impl EditorManager {
    pub fn get(name: &str) -> Result<EditorBox> {
        if name == "clipboard" {
            return Ok(Box::<clipboard::Clipboard>::default());
        }

        if name == "neovim" {
            return Ok(Box::<neovim::Neovim>::default());
        }

        bail!(format!("No editor implemented for {name}"))
    }
}
