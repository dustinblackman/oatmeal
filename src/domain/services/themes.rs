use std::collections::HashMap;
use std::fs::File;
use std::io::BufReader;
use std::io::Cursor;

use anyhow::bail;
use anyhow::Result;
use syntect::highlighting::Theme;
use syntect::highlighting::ThemeSet;

#[derive(Default)]
pub struct Themes {}

impl Themes {
    fn load() -> HashMap<String, String> {
        let payload = include_bytes!(env!("OATMEAL_THEMES_BIN"));
        let themes: HashMap<String, String> = bincode::deserialize_from(&payload[..]).unwrap();
        return themes;
    }

    pub fn list() -> Vec<String> {
        let mut themes = Themes::load()
            .keys()
            .map(|e| return e.to_string())
            .collect::<Vec<String>>();
        themes.sort();

        return themes;
    }

    fn get_from_memory(theme_name: &str) -> Result<Theme> {
        let themes = Themes::load();
        if !themes.contains_key(theme_name) {
            bail!(format!("Theme {theme_name} does not exist in assets"));
        }

        let theme = ThemeSet::load_from_reader(&mut Cursor::new(themes.get(theme_name).unwrap()))?;

        return Ok(theme);
    }

    fn get_from_file(theme_file: &str) -> Result<Theme> {
        let file = File::open(theme_file)?;
        let mut reader = BufReader::new(file);
        let theme = ThemeSet::load_from_reader(&mut reader)?;

        return Ok(theme);
    }

    pub fn get(theme_name: &str, theme_file: &str) -> Result<Theme> {
        if !theme_file.is_empty() {
            return Themes::get_from_file(theme_file);
        }
        return Themes::get_from_memory(theme_name);
    }
}
