#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod hash_assets;
mod readme;

use std::env;
use std::process;

use anyhow::Result;

fn main() -> Result<()> {
    let args: Vec<String> = env::args().collect();
    let last = args.last().unwrap();
    if last == "update-readme" {
        readme::update();
    } else if last == "hash-assets" {
        hash_assets::update()?;
    } else {
        eprintln!("ERROR: No task selected");
        process::exit(1);
    }

    return Ok(());
}
