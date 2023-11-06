use std::fs;
use std::io::Cursor;
use std::path::Path;

use anyhow::anyhow;
use anyhow::Result;
use flate2::read::GzDecoder;
use tar::Archive;
use vergen::EmitBuilder;

fn main() -> Result<()> {
    EmitBuilder::builder().all_git().emit()?;

    let themes = [
        "base16-github",
        "base16-monokai",
        "base16-one-light",
        "base16-onedark",
        "base16-seti",
    ];

    if !Path::new("./themes").exists() {
        fs::create_dir_all("./themes")?;

        let bytes = reqwest::blocking::get("https://github.com/chriskempson/base16-textmate/archive/0e51ddd568bdbe17189ac2a07eb1c5f55727513e.tar.gz")?.bytes()?;
        let tar = GzDecoder::new(Cursor::new(bytes));
        let mut archive = Archive::new(tar);
        archive
            .entries()?
            .filter_map(|e| e.ok())
            .map(|mut entry| -> Result<String> {
                let filename = entry
                    .path()?
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .to_string();

                for theme in themes {
                    if filename.contains(theme) {
                        entry.unpack(&format!("./themes/{filename}"))?;
                        return Ok(filename);
                    }
                }

                Err(anyhow!("No matching theme"))
            })
            .filter_map(|e| e.ok())
            .for_each(|x| println!("> {}", x));
    }

    Ok(())
}
