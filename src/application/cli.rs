use std::io;

use anyhow::Result;
use clap::value_parser;
use clap::Arg;
use clap::ArgAction;
use clap::ArgGroup;
use clap::Command;
use clap_complete::generate;
use clap_complete::Generator;
use clap_complete::Shell;
use dialoguer::theme::ColorfulTheme;
use dialoguer::Select;
use owo_colors::OwoColorize;
use owo_colors::Stream;

use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::models::Session;
use crate::domain::services::actions::help_text;
use crate::domain::services::Sessions;
use crate::domain::services::Themes;

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    std::process::exit(0);
}

fn format_session(session: &Session) -> String {
    let mut res = format!(
        "- (ID: {}) {}, Model: {}",
        session.id.bold(),
        session.timestamp,
        session.state.backend_model,
    );

    if !session.state.editor_language.is_empty() {
        res = format!("{res}, Lang: {}", session.state.editor_language)
    }

    return res;
}

async fn print_sessions_list() -> Result<()> {
    let mut sessions = Sessions::default()
        .list()
        .await?
        .iter()
        .map(|session| {
            return format_session(session);
        })
        .collect::<Vec<String>>();

    sessions.reverse();

    if sessions.is_empty() {
        println!("There are no sessions available. You should start you first one!");
    } else {
        println!("{}", sessions.join("\n"));
    }

    return Ok(());
}

async fn load_config_from_session(session_id: &str) -> Result<()> {
    let session = Sessions::default().load(session_id).await?;
    Config::set(ConfigKey::Backend, &session.state.backend_name);
    Config::set(ConfigKey::Model, &session.state.backend_model);
    Config::set(ConfigKey::SessionID, session_id);

    return Ok(());
}

async fn load_config_from_session_interactive() -> Result<()> {
    let mut sessions = Sessions::default().list().await?;
    sessions.reverse();

    if sessions.is_empty() {
        println!("There are no sessions available. You should start you first one!");
        return Ok(());
    }

    let session_options = sessions
        .iter()
        .map(|session| {
            return format_session(session);
        })
        .collect::<Vec<String>>();

    let idx = Select::with_theme(&ColorfulTheme::default())
        .with_prompt("Which session would you like to load?")
        .default(0)
        .items(&session_options)
        .interact_opt()?
        .unwrap();

    load_config_from_session(&sessions[idx].id).await?;

    return Ok(());
}

fn subcommand_completions() -> Command {
    return Command::new("completions")
        .about("Generates shell completions")
        .arg(
            clap::Arg::new("shell")
                .short('s')
                .long("shell")
                .help("Which shell to generate completions for")
                .action(ArgAction::Set)
                .value_parser(value_parser!(Shell))
                .required(true),
        );
}

fn subcommand_sessions_delete() -> Command {
    return Command::new("delete")
        .about("Delete one or all sessions")
        .arg(
            clap::Arg::new("session-id")
                .short('i')
                .long("id")
                .help("Session ID")
                .num_args(1),
        )
        .arg(
            clap::Arg::new("all")
                .long("all")
                .help("Delete all sessions")
                .num_args(0),
        )
        .group(
            ArgGroup::new("delete-args")
                .args(["session-id", "all"])
                .required(true),
        );
}

fn subcommand_sessions() -> Command {
    return Command::new("sessions")
        .about("Manage past chat sessions")
        .arg_required_else_help(true)
        .subcommand(Command::new("dir").about("Print the sessions cache directory path"))
        .subcommand(Command::new("list").about("List all previous sessions"))
        .subcommand(
            Command::new("open")
                .about("Open a previous session by ID. Omit passing any session ID to load an interactive selection.")
                .arg(
                    clap::Arg::new("session-id")
                        .short('i')
                        .long("id")
                        .help("Session ID")
                        .required(false),
                ),
        )
        .subcommand(subcommand_sessions_delete());
}

fn build() -> Command {
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

    return Command::new("oatmeal")
        .about(about)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(commands_text)
        .arg_required_else_help(false)
        .subcommand(subcommand_completions())
        .subcommand(subcommand_sessions())
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
                .default_value("clipboard")
                .global(true),
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
                .default_value("base16-onedark")
                .global(true),
        )
        .arg(
            Arg::new("theme-file")
                .long("theme-file")
                .env("OATMEAL_THEME_FILE")
                .num_args(1)
                .help(
                    "Absolute path to a TextMate tmTheme to use for code syntax highlighting"
                )
                .global(true),
        )
        .arg(
            Arg::new("openai-url")
                .long("openai-url")
                .env("OATMEAL_OPENAI_URL")
                .num_args(1)
                .help("OpenAI API URL when using the OpenAI backend. Can be swapped to a compatiable proxy")
                .default_value("https://api.openai.com")
                .global(true),
            )
        .arg(
            Arg::new("openai-token")
                .long("openai-token")
                .env("OATMEAL_OPENAI_TOKEN")
                .num_args(1)
                .help("OpenAI API token when using the OpenAI backend.")
                .global(true),
        );
}

pub async fn parse() -> Result<bool> {
    let matches = build().get_matches();

    match matches.subcommand() {
        Some(("completions", subcmd_matches)) => {
            if let Some(completions) = subcmd_matches.get_one::<Shell>("shell").copied() {
                let mut app = build();
                print_completions(completions, &mut app);
            }
        }
        Some(("sessions", subcmd_matches)) => {
            match subcmd_matches.subcommand() {
                Some(("dir", _)) => {
                    let dir = Sessions::default().cache_dir.to_string_lossy().to_string();
                    println!("{dir}");
                    return Ok(false);
                }
                Some(("list", _)) => {
                    print_sessions_list().await?;
                    return Ok(false);
                }
                Some(("open", open_matches)) => {
                    if let Some(session_id) = open_matches.get_one::<String>("session-id") {
                        load_config_from_session(session_id).await?;
                    } else {
                        load_config_from_session_interactive().await?;
                    }
                }
                Some(("delete", delete_matches)) => {
                    if let Some(session_id) = delete_matches.get_one::<String>("session-id") {
                        Sessions::default().delete(session_id).await?;
                        println!("Deleted session {session_id}");
                    } else if delete_matches.get_one::<bool>("all").is_some() {
                        Sessions::default().delete_all().await?;
                        println!("Deleted all sessions");
                    } else {
                        subcommand_sessions_delete().print_long_help()?;
                    }
                    return Ok(false);
                }
                _ => {
                    subcommand_sessions().print_long_help()?;
                    return Ok(false);
                }
            }
        }
        _ => {
            Config::set(
                ConfigKey::Backend,
                matches.get_one::<String>("backend").unwrap(),
            );
            Config::set(
                ConfigKey::Model,
                matches.get_one::<String>("model").unwrap(),
            );
        }
    }

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

    return Ok(true);
}
