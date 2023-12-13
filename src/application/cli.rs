use std::env;
use std::io;

use anyhow::Result;
use clap::builder::PossibleValuesParser;
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
use strum::VariantNames;
use yansi::Paint;

use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::models::BackendName;
use crate::domain::models::EditorName;
use crate::domain::models::Session;
use crate::domain::services::actions::help_text;
use crate::domain::services::Sessions;
use crate::domain::services::Syntaxes;
use crate::domain::services::Themes;

fn print_completions<G: Generator>(gen: G, cmd: &mut Command) {
    generate(gen, cmd, cmd.get_name().to_string(), &mut io::stdout());
    std::process::exit(0);
}

fn format_session(session: &Session) -> String {
    let mut res = format!(
        "- (ID: {}) {}, Model: {}",
        session.id, session.timestamp, session.state.backend_model,
    );

    if !session.state.editor_language.is_empty() {
        res = format!("{res}, Lang: {}", session.state.editor_language)
    }

    if !session.state.messages.is_empty() {
        let mut line = session.state.messages[0]
            .text
            .split('\n')
            .collect::<Vec<_>>()[0]
            .to_string();

        if line.len() >= 70 {
            line = format!("{}...", &line[..67]);
        }
        res = format!("{res}, {line}");
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
        println!("There are no sessions available. You should start your first one!");
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
        println!("There are no sessions available. You should start your first one!");
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

fn subcommand_debug() -> Command {
    return Command::new("debug")
        .about("Debug helpers for Oatmeal")
        .hide(true)
        .subcommand(
            Command::new("syntaxes").about("List all supported code highlighting languages.")
        )
        .subcommand(
            Command::new("themes").about("List all supported code highlighting themes.")
        )
        .subcommand(
            Command::new("log-path").about("Output path to debug log file generated when running Oatmeal with environment variable RUST_LOG=oatmeal")
        )
        .subcommand(
            Command::new("enum-config").about("List all config keys as strings.")
        );
}

fn subcommand_completions() -> Command {
    return Command::new("completions")
        .about("Generates shell completions")
        .arg(
            clap::Arg::new("shell")
                .short('s')
                .long("shell")
                .help("Which shell to generate completions for.")
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

fn arg_backend() -> Arg {
    return Arg::new("backend")
        .short('b')
        .long("backend")
        .env("OATMEAL_BACKEND")
        .num_args(1)
        .help("The initial backend hosting a model to connect to.")
        .value_parser(PossibleValuesParser::new(BackendName::VARIANTS))
        .default_value("ollama");
}

fn arg_backend_health_check_timeout() -> Arg {
    return Arg::new("backend-health-check-timeout")
        .long("backend-health-check-timeout")
        .env("OATMEAL_BACKEND_HEALTH_CHECK_TIMEOUT")
        .num_args(1)
        .help(
            "Time to wait in milliseconds before timing out when doing a healthcheck for a backend",
        )
        .default_value("1000");
}

fn arg_model() -> Arg {
    return Arg::new("model")
        .short('m')
        .long("model")
        .env("OATMEAL_MODEL")
        .num_args(1)
        .help("The initial model on a backend to consume")
        .default_value("llama2:latest");
}

fn subcommand_chat() -> Command {
    return Command::new("chat")
        .about("Start a new chat session")
        .arg(arg_backend())
        .arg(arg_backend_health_check_timeout())
        .arg(arg_model());
}

fn subcommand_sessions() -> Command {
    return Command::new("sessions")
        .about("Manage past chat sessions")
        .arg_required_else_help(true)
        .subcommand(Command::new("dir").about("Print the sessions cache directory path."))
        .subcommand(Command::new("list").about("List all previous sessions with their ids and models."))
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
                return Paint::new(format!("CHAT {line}"))
                    .underline()
                    .bold()
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

    let themes = Themes::list();

    return Command::new("oatmeal")
        .about(about)
        .author(env!("CARGO_PKG_AUTHORS"))
        .version(env!("CARGO_PKG_VERSION"))
        .after_help(commands_text)
        .arg_required_else_help(false)
        .subcommand(subcommand_chat())
        .subcommand(subcommand_completions())
        .subcommand(subcommand_debug())
        .subcommand(subcommand_sessions())
        .arg(arg_backend())
        .arg(arg_backend_health_check_timeout())
        .arg(arg_model())
        .arg(
            Arg::new("editor")
                .short('e')
                .long("editor")
                .env("OATMEAL_EDITOR")
                .num_args(1)
                .help("The editor to integrate with.")
                .value_parser(PossibleValuesParser::new(EditorName::VARIANTS))
                .default_value("clipboard")
                .global(true),
        )
        .arg(
            Arg::new("theme")
                .short('t')
                .long("theme")
                .env("OATMEAL_THEME")
                .num_args(1)
                .help("Sets code syntax highlighting theme.")
                .value_parser(PossibleValuesParser::new(themes))
                .default_value("base16-onedark")
                .global(true),
        )
        .arg(
            Arg::new("theme-file")
                .long("theme-file")
                .env("OATMEAL_THEME_FILE")
                .num_args(1)
                .help(
                    "Absolute path to a TextMate tmTheme to use for code syntax highlighting."
                )
                .global(true),
        )
        .arg(
            Arg::new("ollama-url")
                .long("ollama-url")
                .env("OATMEAL_OLLAMA_URL")
                .num_args(1)
                .help("Ollama API URL when using the Ollama backend.")
                .default_value("http://localhost:11434")
                .global(true),
        )
        .arg(
            Arg::new("openai-url")
                .long("openai-url")
                .env("OATMEAL_OPENAI_URL")
                .num_args(1)
                .help("OpenAI API URL when using the OpenAI backend. Can be swapped to a compatiable proxy.")
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
        Some(("debug", debug_matches)) => {
            match debug_matches.subcommand() {
                Some(("syntaxes", _)) => {
                    println!("{}", Syntaxes::list().join("\n"));
                    return Ok(false);
                }
                Some(("themes", _)) => {
                    println!("{}", Themes::list().join("\n"));
                    return Ok(false);
                }
                Some(("log-path", _)) => {
                    let log_path = dirs::cache_dir().unwrap().join("oatmeal/debug.log");
                    println!("{}", log_path.to_str().unwrap());
                    return Ok(false);
                }
                Some(("enum-config", _)) => {
                    let res = ConfigKey::VARIANTS.join("\n");
                    println!("{}", res);
                    return Ok(false);
                }
                _ => {
                    subcommand_debug().print_long_help()?;
                    return Ok(false);
                }
            }
        }
        Some(("chat", subcmd_matches)) => {
            Config::set(
                ConfigKey::Backend,
                subcmd_matches.get_one::<String>("backend").unwrap(),
            );
            Config::set(
                ConfigKey::BackendHealthCheckTimeout,
                matches
                    .get_one::<String>("backend-health-check-timeout")
                    .unwrap(),
            );
            Config::set(
                ConfigKey::Model,
                subcmd_matches.get_one::<String>("model").unwrap(),
            );
        }
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
                ConfigKey::BackendHealthCheckTimeout,
                matches
                    .get_one::<String>("backend-health-check-timeout")
                    .unwrap(),
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
        ConfigKey::OllamaURL,
        matches.get_one::<String>("ollama-url").unwrap(),
    );
    Config::set(
        ConfigKey::OpenAiURL,
        matches.get_one::<String>("openai-url").unwrap(),
    );

    let mut user = env::var("USER").unwrap_or_else(|_| return "".to_string());
    if user.is_empty() {
        user = "User".to_string();
    }
    Config::set(ConfigKey::Username, &user);

    if let Some(theme_file) = matches.get_one::<String>("theme-file") {
        Config::set(ConfigKey::ThemeFile, theme_file);
    }

    if let Some(openai_token) = matches.get_one::<String>("openai-token") {
        Config::set(ConfigKey::OpenAiToken, openai_token);
    }

    tracing::debug!(
        username = Config::get(ConfigKey::Username),
        backend = Config::get(ConfigKey::Backend),
        editor = Config::get(ConfigKey::Editor),
        model = Config::get(ConfigKey::Model),
        theme = Config::get(ConfigKey::Theme),
        theme_file = Config::get(ConfigKey::ThemeFile),
        "config"
    );

    return Ok(true);
}
