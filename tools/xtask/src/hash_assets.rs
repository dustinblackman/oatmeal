use std::fs;
use std::process;

use anyhow::Result;
use toml_edit::value;
use toml_edit::Document;

fn hash(url: String) -> Result<String> {
    println!("Hashing {url}");

    let mut child = process::Command::new("docker");
    child.arg("run")
        .arg("--rm")
        .arg("nixos/nix:latest")
        .arg("bash")
        .arg("-c")
        .arg(format!("mkdir /tmp/extract && curl -s -L {url} | tar xz --strip-components=1 -C /tmp/extract && nix --extra-experimental-features nix-command hash path /tmp/extract/"))
        .env("DOCKER_DEFAULT_PLATFORM", "linux/amd64")
        .env("NO_COLOR", "1");

    let output = child.output()?;
    let res = String::from_utf8(output.stdout)?;
    let hash = res
        .split('\n')
        .find(|e| return e.starts_with("sha256-"))
        .unwrap()
        .to_string();

    return Ok(hash);
}

pub fn update(force: bool) -> Result<()> {
    let toml_str = fs::read_to_string("./assets.toml")?;
    let mut doc = toml_str.parse::<Document>()?;

    for key in ["syntaxes", "themes"] {
        for entry in doc[key].as_array_of_tables_mut().unwrap().iter_mut() {
            let nix_hash = entry["nix-hash"].as_str().unwrap();
            if !nix_hash.is_empty() && !force {
                continue;
            }

            let url = format!(
                "https://github.com/{owner}/{repo}/archive/{rev}.tar.gz",
                owner = entry["owner"].as_str().unwrap(),
                repo = entry["repo"].as_str().unwrap(),
                rev = entry["rev"].as_str().unwrap()
            );

            entry["nix-hash"] = value(hash(url)?);
        }
    }

    fs::write("./assets.toml", doc.to_string())?;
    println!("Done");

    return Ok(());
}
