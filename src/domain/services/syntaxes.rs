use syntect::parsing::SyntaxSet;

pub struct Syntaxes {}

impl Syntaxes {
    pub fn load() -> SyntaxSet {
        let payload = include_bytes!("../../../.cache/syntaxes/syntaxes.bin");
        let syntax_set: SyntaxSet = bincode::deserialize_from(&payload[..]).unwrap();
        return syntax_set;
    }
}
