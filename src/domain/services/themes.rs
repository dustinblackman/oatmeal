use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;

use anyhow::bail;
use anyhow::Result;
use rust_embed::RustEmbed;
use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;

#[derive(RustEmbed)]
#[folder = ".cache/themes/"]
#[include = "*.tmTheme"]
struct Assets;

#[derive(Default)]
pub struct Themes {}

impl Themes {
    pub fn list() -> Vec<String> {
        return Assets::iter()
            .map(|file| {
                return file
                    .split('.')
                    .collect::<Vec<_>>()
                    .first()
                    .unwrap()
                    .to_string();
            })
            .collect::<Vec<_>>();
    }

    fn load_from_memory(theme_name: &str) -> Result<Theme> {
        let file_op = Assets::get(&format!("{theme_name}.tmTheme"));
        if file_op.is_none() {
            bail!(format!("Theme {theme_name} does not exist in assets"));
        }
        let file = file_op.unwrap();
        let theme = ThemeSet::load_from_reader(&mut Cursor::new(file.data.as_ref()))?;

        return Ok(theme);
    }

    fn load_from_file(theme_file: &str) -> Result<Theme> {
        let file = File::open(theme_file)?;
        let mut reader = BufReader::new(file);
        let theme = ThemeSet::load_from_reader(&mut reader)?;

        return Ok(theme);
    }

    pub fn load(theme_name: &str, theme_file: &str) -> Result<Theme> {
        if !theme_file.is_empty() {
            return Themes::load_from_file(theme_file);
        }
        return Themes::load_from_memory(theme_name);
    }
}
