use std::env;
use std::fs;
use std::io::Write;
use std::process;

fn update_readme() {
    let output_help = String::from_utf8(
        process::Command::new("./target/debug/oatmeal")
            .arg("--help")
            .env("NO_COLOR", "1")
            .env("OATMEAL_THEME", "")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();

    let version_res = String::from_utf8(
        process::Command::new("./target/debug/oatmeal")
            .arg("--version")
            .output()
            .unwrap()
            .stdout,
    )
    .unwrap();
    let version = version_res.split(' ').collect::<Vec<&str>>()[1];

    let mut readme = fs::read_to_string("./README.md").unwrap();
    let start_help = readme.find("<!-- command-help start -->").unwrap();
    let end_help = readme.find("<!-- command-help end -->").unwrap();
    readme.replace_range(
        start_help..end_help,
        &format!("<!-- command-help start -->\n```\n{output_help}```\n"),
    );

    let start_choco = readme.find("<!-- choco-install start -->").unwrap();
    let end_choco = readme.find("<!-- choco-install end -->").unwrap();
    readme.replace_range(
        start_choco..end_choco,
        &format!(
            "<!-- choco-install start -->\n```sh\nchoco install oatmeal --version={version}```\n"
        ),
    );

    let mut f = fs::File::create("./README.md").unwrap();
    f.write_all(readme.as_bytes()).unwrap();
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.last().unwrap() == "update-readme" {
        // Check args in the future.
        update_readme();
    } else {
        eprintln!("ERROR: No task selected");
        process::exit(1);
    }
}
