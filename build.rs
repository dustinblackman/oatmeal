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
use syntect::parsing::SyntaxSetBuilder;
use tar::Archive;
use vergen::EmitBuilder;

#[derive(Debug)]
struct SyntaxDownload<'a> {
    name: &'a str,
    url: &'a str,
    files: Vec<&'a str>,
    keep_folders: bool,
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

fn download_files(
    download_folder: PathBuf,
    url: &str,
    files: Vec<&str>,
    keep_folders: bool,
) -> Result<()> {
    fs::create_dir_all(download_folder.clone())?;

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

            for req_file in files.clone().into_iter() {
                if glob_match(&format!("*/{req_file}"), &filepath) {
                    if keep_folders {
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
                            entry.unpack(download_folder.clone().join(filename.clone()))?;
                        } else {
                            fs::create_dir_all(download_folder.clone().join(dir.clone()))?;
                            entry.unpack(
                                download_folder.clone().join(format!("{dir}/{filename}")),
                            )?;
                        }
                    } else {
                        entry.unpack(download_folder.clone().join(filename.clone()))?;
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

fn get_syntaxes() -> Result<()> {
    let out_dir = get_cache_dir()?.join("syntaxes");
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

    let downloads: Vec<SyntaxDownload> = vec![
        SyntaxDownload {
            name: "sublime-packages",
            url: "https://github.com/sublimehq/Packages/archive/759d6eed9b4beed87e602a23303a121c3a6c2fb3.tar.gz",
            files: vec!["LICENSE", "*/LICENSE", "*/LICENSE.*", "*/*.sublime-syntax"],
            keep_folders: true
        },
        SyntaxDownload {
            name: "bat",
            url:
                "https://github.com/sharkdp/bat/archive/7658334645936d2a956fb19aa96e6fca849cb754.tar.gz",
            files: vec!["LICENSE-MIT", "assets/syntaxes/02_Extra/*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "GraphQL-SublimeText3",
            url:
                "https://github.com/dncrews/GraphQL-SublimeText3/archive/9b6f6d0a86d7e7ef1d44490b107472af7fb4ffaf.tar.gz",
            files: vec!["LICENSE", "*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "protobuf-syntax-highlighting",
            url:
                "https://github.com/VcamX/protobuf-syntax-highlighting/archive/726e21d74dac23cbb036f2fbbd626decdc954060.tar.gz",
            files: vec!["LICENSE", "*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "sublime-zig-language",
            url:
                "https://github.com/ziglang/sublime-zig-language/archive/1a4a38445fec495817625bafbeb01e79c44abcba.tar.gz",
            files: vec!["LICENSE", "Syntaxes/*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "Terraform.tmLanguage",
            url:
                "https://github.com/alexlouden/Terraform.tmLanguage/archive/54d8350c3c5929c921ea7561c932aa15e7d96c48.tar.gz",
            files: vec!["LICENSE", "*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "sublime_toml_highlighting",
            url:
                "https://github.com/jasonwilliams/sublime_toml_highlighting/archive/fd0bf3e5d6c9e6397c0dc9639a0514d9bf55b800.tar.gz",
            files: vec!["LICENSE", "*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "elixir-sublime-syntax",
            url:
                "https://github.com/princemaple/elixir-sublime-syntax/archive/4fb01891dd17434dde42887bc821917a30f4e010.tar.gz",
            files: vec!["LICENSE", "*.sublime-syntax"],
            keep_folders: false
        },
        SyntaxDownload {
            name: "sublime-text-gleam",
            url:
                "https://github.com/digitalcora/sublime-text-gleam/archive/0b032f78c9c4aec1c598da1d25c67ca21fa8c381.tar.gz",
            files: vec!["LICENSE", "package/*.sublime-syntax"],
            keep_folders: false
        },
    ];

    for download in downloads {
        download_files(
            out_dir.join(download.name),
            download.url,
            download.files,
            download.keep_folders,
        )?;
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

fn get_themes() -> Result<()> {
    let out_dir = get_cache_dir()?.join("themes");
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

    let files = vec![
        "LICENSE.md",
        "Themes/base16-github.tmTheme",
        "Themes/base16-monokai.tmTheme",
        "Themes/base16-one-light.tmTheme",
        "Themes/base16-onedark.tmTheme",
        "Themes/base16-seti.tmTheme",
    ];
    download_files(out_dir.clone(), "https://github.com/chriskempson/base16-textmate/archive/0e51ddd568bdbe17189ac2a07eb1c5f55727513e.tar.gz", files.clone(), true)?;

    let mut themes_map = HashMap::new();
    for e in files {
        if !e.ends_with(".tmTheme") {
            continue;
        }

        let theme_path = out_dir.join(e);
        let file_name = theme_path
            .file_name()
            .unwrap()
            .to_string_lossy()
            .to_string();
        let theme_name = file_name.split('.').collect::<Vec<_>>()[0].to_string();
        themes_map.insert(theme_name, fs::read_to_string(&theme_path)?);
    }

    let mut payload = vec![];
    bincode::serialize_into(&mut payload, &themes_map)?;

    let mut file = fs::File::create(themes_bin)?;
    file.write_all(&payload)?;

    return Ok(());
}

fn main() -> Result<()> {
    EmitBuilder::builder().all_git().emit()?;
    get_themes()?;
    get_syntaxes()?;

    return Ok(());
}
