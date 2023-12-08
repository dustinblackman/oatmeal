use once_cell::sync::Lazy;
use ratatui::style::Color;
use syntect::parsing::SyntaxReference;
use syntect::parsing::SyntaxSet;

pub static SYNTAX_SET: Lazy<SyntaxSet> = Lazy::new(Syntaxes::load);

pub struct Syntaxes {}

impl Syntaxes {
    fn load() -> SyntaxSet {
        let payload = include_bytes!(env!("OATMEAL_SYNTAX_BIN"));
        let syntax_set: SyntaxSet = bincode::deserialize_from(&payload[..]).unwrap();
        return syntax_set;
    }

    pub fn get(name: &str) -> &SyntaxReference {
        if let Some(syntax) = SYNTAX_SET.find_syntax_by_extension(name) {
            return syntax;
        }

        if let Some(syntax) = SYNTAX_SET.find_syntax_by_name(name) {
            return syntax;
        }

        if let Some(syntax) = SYNTAX_SET.find_syntax_by_token(name) {
            return syntax;
        }

        return SYNTAX_SET.find_syntax_plain_text();
    }

    pub fn list() -> Vec<String> {
        let mut syntaxes = SYNTAX_SET
            .syntaxes()
            .iter()
            .map(|e| return e.name.to_string())
            .collect::<Vec<String>>();
        syntaxes.sort();
        return syntaxes;
    }

    pub fn translate_colour(syntect_color: syntect::highlighting::Color) -> Option<Color> {
        match syntect_color {
            syntect::highlighting::Color { r, g, b, a } if a > 0 => {
                return Some(Color::Rgb(r, g, b))
            }
            _ => return None,
        }
    }
}
