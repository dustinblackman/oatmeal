#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

use std::collections::HashMap;
use std::env;
use std::fs;
use std::io::prelude::*;
use std::io::Cursor;
use std::path::PathBuf;

use anyhow::anyhow;
use anyhow::Result;
use flate2::read::GzDecoder;
use glob_match::glob_match;
use serde::Deserialize;
use serde::Serialize;
use syntect::parsing::SyntaxSetBuilder;
use tar::Archive;
use vergen::EmitBuilder;
use walkdir::WalkDir;

#[derive(Clone, Deserialize, Serialize)]
struct Asset {
    owner: String,
    repo: String,
    rev: String,
    files: Vec<String>,
    #[serde(alias = "nix-hash")]
    nix_hash: String,
}

#[derive(Deserialize, Serialize)]
struct Assets {
    syntaxes: Vec<Asset>,
    themes: Vec<Asset>,
}

pub fn get_project_root() -> Result<PathBuf> {
    let path = env::current_dir()?;
    let path_ancestors = path.as_path().ancestors();

    for p in path_ancestors {
        let has_cargo = fs::read_dir(p)?.any(|p| return p.unwrap().file_name() == *"Cargo.lock");

        if has_cargo {
            return Ok(PathBuf::from(p));
        }
    }

    return Err(anyhow!("Root directory for rust project not found."));
}

fn get_cache_dir() -> Result<PathBuf> {
    let out_dir = env::var("OUT_DIR").unwrap();
    if env::var("OPT_LEVEL").unwrap_or_else(|_| return "0".to_string()) == "3"
        || out_dir.contains("target/package/")
    {
        return Ok(PathBuf::from(out_dir).join(".cache"));
    }

    return Ok(get_project_root()?.join(".cache"));
}

fn download_files(download_folder: PathBuf, asset: Asset) -> Result<()> {
    let output_folder = download_folder.join(format!("{}-{}", asset.owner, asset.repo));
    fs::create_dir_all(output_folder.clone())?;

    let url = format!(
        "https://github.com/{owner}/{repo}/archive/{rev}.tar.gz",
        owner = asset.owner,
        repo = asset.repo,
        rev = asset.rev
    );
    let bytes = reqwest::blocking::get(url)?.bytes()?;
    let tar = GzDecoder::new(Cursor::new(bytes));
    let mut archive = Archive::new(tar);

    archive
        .entries()?
        .filter_map(|e| return e.ok())
        .map(|mut entry| -> Result<String> {
            let filepath = entry.path()?.to_string_lossy().to_string();
            let filename = entry
                .path()?
                .file_name()
                .unwrap()
                .to_string_lossy()
                .to_string();

            for req_file in asset.files.clone().into_iter() {
                if glob_match(&format!("*/{req_file}"), &filepath) {
                    let dir = entry
                        .path()?
                        .parent()
                        .unwrap()
                        .components()
                        .enumerate()
                        .filter_map(|(idx, p)| {
                            if idx == 0 {
                                return None;
                            }
                            return Some(p);
                        })
                        .collect::<PathBuf>()
                        .to_string_lossy()
                        .to_string();

                    if dir.is_empty() {
                        entry.unpack(output_folder.clone().join(filename.clone()))?;
                    } else {
                        fs::create_dir_all(output_folder.clone().join(dir.clone()))?;
                        entry.unpack(output_folder.clone().join(format!("{dir}/{filename}")))?;
                    }

                    return Ok(filename);
                }
            }

            return Err(anyhow!("No matching file"));
        })
        .filter_map(|e| return e.ok())
        .for_each(|x| println!("> {}", x));

    return Ok(());
}

fn get_syntaxes(syntaxes: Vec<Asset>) -> Result<()> {
    let mut out_dir = get_cache_dir()?.join("syntaxes");
    if let Ok(env_syntaxes_dir) = env::var("OATMEAL_BUILD_DOWNLOADED_SYNTAXES_DIR") {
        if !env_syntaxes_dir.is_empty() {
            out_dir = PathBuf::from(env_syntaxes_dir);
        }
    }

    let syntax_bin = out_dir.join("syntaxes.bin");
    println!(
        "cargo:rustc-env=OATMEAL_SYNTAX_BIN={}",
        syntax_bin.clone().to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        syntax_bin.clone().to_str().unwrap()
    );

    if syntax_bin.exists() {
        return Ok(());
    }

    // If the env is set, assume scripts externally have already downloaded
    // syntaxes.
    if env::var("OATMEAL_BUILD_DOWNLOADED_SYNTAXES_DIR").is_err() {
        for asset in syntaxes {
            download_files(out_dir.clone(), asset)?;
        }
    }

    let mut builder = SyntaxSetBuilder::new();
    builder.add_plain_text_syntax();
    builder.add_from_folder(out_dir.clone(), true)?;

    let syntax_set = builder.build();
    let mut payload = vec![];
    bincode::serialize_into(&mut payload, &syntax_set)?;

    let mut file = fs::File::create(syntax_bin)?;
    file.write_all(&payload)?;

    return Ok(());
}

fn get_themes(themes: Vec<Asset>) -> Result<()> {
    let mut out_dir = get_cache_dir()?.join("themes");
    if let Ok(env_themes_dir) = env::var("OATMEAL_BUILD_DOWNLOADED_THEMES_DIR") {
        if !env_themes_dir.is_empty() {
            out_dir = PathBuf::from(env_themes_dir);
        }
    }

    let themes_bin = out_dir.join("themes.bin");
    println!(
        "cargo:rustc-env=OATMEAL_THEMES_BIN={}",
        themes_bin.clone().to_str().unwrap()
    );
    println!(
        "cargo:rerun-if-changed={}",
        themes_bin.clone().to_str().unwrap()
    );
    if themes_bin.exists() {
        return Ok(());
    }

    // If the env is set, assume scripts externally have already downloaded themes.
    if env::var("OATMEAL_BUILD_DOWNLOADED_THEMES_DIR").is_err() {
        for asset in themes.clone() {
            download_files(out_dir.clone(), asset)?;
        }
    }

    let theme_files = WalkDir::new(out_dir)
        .into_iter()
        .filter_map(|e| {
            if e.is_err() {
                return None;
            }
            let p = e.ok();
            if !p
                .clone()
                .unwrap()
                .path()
                .to_str()
                .unwrap()
                .ends_with(".tmTheme")
            {
                return None;
            }
            return Some(p.unwrap().path().to_owned());
        })
        .collect::<Vec<PathBuf>>();

    let mut themes_map = HashMap::new();

    for theme in themes {
        for file in theme.files {
            if !file.ends_with(".tmTheme") {
                continue;
            }

            let file_path = theme_files
                .iter()
                .find(|e| {
                    let path_str = e.to_string_lossy().to_string();
                    return path_str.ends_with(&file.replace("Themes/", ""));
                })
                .unwrap();
            let file_name = file_path.file_name().unwrap().to_string_lossy().to_string();

            let theme_name = file_name.split('.').collect::<Vec<_>>()[0].to_string();
            themes_map.insert(theme_name, fs::read_to_string(file_path)?);
        }
    }

    let mut payload = vec![];
    bincode::serialize_into(&mut payload, &themes_map)?;

    let mut file = fs::File::create(themes_bin)?;
    file.write_all(&payload)?;

    return Ok(());
}

fn main() -> Result<()> {
    EmitBuilder::builder().all_git().emit()?;

    let assets: Assets = toml::from_str(&fs::read_to_string("./assets.toml")?).unwrap();
    get_themes(assets.themes)?;
    get_syntaxes(assets.syntaxes)?;

    return Ok(());
}
