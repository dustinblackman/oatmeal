use clap::Arg;
use clap::Command;
use owo_colors::OwoColorize;
use owo_colors::Stream;

use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::services::actions::help_text;
use crate::domain::services::Themes;

pub fn parse() {
    let commands_text = help_text()
        .split('\n')
        .map(|line| {
            if line.starts_with('-') {
                return format!("  {line}");
            }
            if line.starts_with("COMMANDS:")
                || line.starts_with("HOTKEYS:")
                || line.starts_with("CODE ACTIONS:")
            {
                return format!("CHAT {line}")
                    .if_supports_color(Stream::Stdout, |text| {
                        return text.underline().bold().to_string();
                    })
                    .to_string();
            }
            return line.to_string();
        })
        .collect::<Vec<String>>()
        .join("\n");

    let about = format!(
        "{}\n\nVersion: {}\nCommit: {}",
        env!("CARGO_PKG_DESCRIPTION"),
        env!("CARGO_PKG_VERSION"),
        env!("VERGEN_GIT_DESCRIBE")
    );
    let themes = Themes::list().join(", ");

    let app = Command::new("oatmeal")
        .about(about)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(commands_text)
        .arg_required_else_help(false)
        .arg(
            Arg::new("backend")
                .short('b')
                .long("backend")
                .env("OATMEAL_BACKEND")
                .num_args(1)
                .help(
                    "The initial backend hosting a model to connect to. [Possible values: ollama, openai]",
                )
                .default_value("ollama"),
        )
        .arg(
            Arg::new("model")
                .short('m')
                .long("model")
                .env("OATMEAL_MODEL")
                .num_args(1)
                .help("The initial model on a backend to consume")
                .default_value("llama2:latest"),
        )
        .arg(
            Arg::new("editor")
                .short('e')
                .long("editor")
                .env("OATMEAL_EDITOR")
                .num_args(1)
                .help("The editor to integrate with. [Possible values: clipboard, neovim]")
                .default_value("clipboard"),
        )
        .arg(
            Arg::new("theme")
                .short('t')
                .long("theme")
                .env("OATMEAL_THEME")
                .num_args(1)
                .help(format!(
                    "Sets code syntax highlighting theme. [Possible values: {themes}]"
                ))
                .default_value("base16-onedark"),
        )
        .arg(
            Arg::new("theme-file")
                .long("theme-file")
                .env("OATMEAL_THEME_FILE")
                .num_args(1)
                .help(
                    "Absolute path to a TextMate tmTheme to use for code syntax highlighting"
                ),
        ).arg(
            Arg::new("openai-url")
                .long("openai-url")
                .env("OATMEAL_OPENAI_URL")
                .num_args(1)
                .help("OpenAI API URL when using the OpenAI backend. Can be swapped to a compatiable proxy")
                .default_value("https://api.openai.com"),
            )
            .arg(
            Arg::new("openai-token")
                .long("openai-token")
                .env("OATMEAL_OPENAI_TOKEN")
                .num_args(1)
                .help("OpenAI API token when using the OpenAI backend."),
            );

    let matches = app.get_matches();
    Config::set(
        ConfigKey::Backend,
        matches.get_one::<String>("backend").unwrap(),
    );
    Config::set(
        ConfigKey::Model,
        matches.get_one::<String>("model").unwrap(),
    );
    Config::set(
        ConfigKey::Editor,
        matches.get_one::<String>("editor").unwrap(),
    );
    Config::set(
        ConfigKey::Theme,
        matches.get_one::<String>("theme").unwrap(),
    );
    Config::set(
        ConfigKey::OpenAIURL,
        matches.get_one::<String>("openai-url").unwrap(),
    );

    if let Some(theme_file) = matches.get_one::<String>("theme-file") {
        Config::set(ConfigKey::ThemeFile, theme_file);
    }

    if let Some(openai_token) = matches.get_one::<String>("openai-token") {
        Config::set(ConfigKey::OpenAIToken, openai_token);
    }
}
