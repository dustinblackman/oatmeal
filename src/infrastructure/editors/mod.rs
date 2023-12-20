pub mod clipboard;
pub mod neovim;
pub mod noop;

use anyhow::bail;
use anyhow::Result;

use crate::domain::models::EditorBox;
use crate::domain::models::EditorName;

pub struct EditorManager {}

impl EditorManager {
    pub fn get(name: EditorName) -> Result<EditorBox> {
        if name == EditorName::Clipboard {
            return Ok(Box::<clipboard::Clipboard>::default());
        }

        if name == EditorName::Neovim {
            return Ok(Box::<neovim::Neovim>::default());
        }

        if name == EditorName::None {
            return Ok(Box::<noop::NoopEditor>::default());
        }

        bail!(format!("No editor implemented for {name}"))
    }
}
