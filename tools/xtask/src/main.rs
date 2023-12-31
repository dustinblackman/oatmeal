#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod hash_assets;
mod readme;

use std::env;
use std::process;

use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    if args.contains(&"update-readme".to_string()) {
        readme::update();
    } else if args.contains(&"hash-assets".to_string()) {
        hash_assets::update(args.contains(&"force".to_string()))?;
    } else {
        eprintln!("ERROR: No task selected");
        process::exit(1);
    }

    return Ok(());
}
