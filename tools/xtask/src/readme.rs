use std::env;
use std::fs;
use std::io::Write;
use std::process;

fn cmd(args: Vec<&str>) -> String {
    let mut child = process::Command::new("./target/debug/oatmeal");
    for arg in args {
        child.arg(arg);
    }

    for (key, _) in env::vars() {
        if key.starts_with("OATMEAL_") {
            child.env(key, "");
        }
    }

    return String::from_utf8(child.env("NO_COLOR", "1").output().unwrap().stdout).unwrap();
}

pub fn update() {
    let output_help = cmd(vec!["--help"]);
    let output_help_sessions = cmd(vec!["sessions", "--help"])
        .split("Options:")
        .next()
        .unwrap()
        .trim()
        .to_string();
    let output_config = cmd(vec!["config", "--help"])
        .split("Options:")
        .next()
        .unwrap()
        .trim()
        .to_string();
    let version_res = cmd(vec!["--version"]);

    let version = version_res.split(' ').collect::<Vec<&str>>()[1].trim();

    let mut readme = fs::read_to_string("./README.md").unwrap();

    let start_help = readme.find("<!-- command-help start -->").unwrap();
    let end_help = readme.find("<!-- command-help end -->").unwrap();
    readme.replace_range(
        start_help..end_help,
        &format!("<!-- command-help start -->\n```\n{output_help}```\n"),
    );

    let start_help_sessions = readme.find("<!-- command-help-sessions start -->").unwrap();
    let end_help_sessions = readme.find("<!-- command-help-sessions end -->").unwrap();
    readme.replace_range(
        start_help_sessions..end_help_sessions,
        &format!("<!-- command-help-sessions start -->\n```\n{output_help_sessions}\n```\n"),
    );

    let start_help_config = readme.find("<!-- command-config start -->").unwrap();
    let end_help_config = readme.find("<!-- command-config end -->").unwrap();
    readme.replace_range(
        start_help_config..end_help_config,
        &format!("<!-- command-config start -->\n```\n{output_config}\n```\n"),
    );

    let start_choco = readme.find("<!-- choco-install start -->").unwrap();
    let end_choco = readme.find("<!-- choco-install end -->").unwrap();
    readme.replace_range(
        start_choco..end_choco,
        &format!(
            "<!-- choco-install start -->\n```sh\nchoco install oatmeal --version={version}\n```\n"
        ),
    );

    let start_alpine = readme.find("<!-- alpine-install start -->").unwrap();
    let end_alpine = readme.find("<!-- alpine-install end -->").unwrap();
    readme.replace_range(
        start_alpine..end_alpine,
        &format!(r#"
<!-- alpine-install start -->

```sh
arch=$(uname -a | grep -q aarch64 && echo 'arm64' || echo 'amd64')
curl -L -o oatmeal.apk "https://github.com/dustinblackman/oatmeal/releases/download/v{version}/oatmeal_{version}_linux_${{arch}}.apk"
apk add --allow-untrusted ./oatmeal.apk
```
"#)
    );

    readme = readme.replace(&env::var("HOME").unwrap(), "~");

    let mut f = fs::File::create("./README.md").unwrap();
    f.write_all(readme.as_bytes()).unwrap();
}
