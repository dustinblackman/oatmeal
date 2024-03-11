use std::env;
use std::io;
use std::path;

use anyhow::bail;
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
use tokio::fs;
use tokio::io::AsyncWriteExt;
use yansi::Paint;

use crate::configuration::Config;
use crate::configuration::ConfigKey;
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

async fn create_config_file() -> Result<()> {
    let config_file_path_str = Config::default(ConfigKey::ConfigFile);
    let config_file_path = path::PathBuf::from(&config_file_path_str);
    if config_file_path.exists() {
        bail!(format!(
            "Config file already exists at {config_file_path_str}"
        ));
    }

    if !config_file_path.parent().unwrap().exists() {
        fs::create_dir_all(config_file_path.parent().unwrap()).await?;
    }

    let mut file = fs::File::create(config_file_path.clone()).await?;
    file.write_all(Config::serialize_default(build()).as_bytes())
        .await?;

    let config_path_display = config_file_path.as_os_str().to_str().unwrap();
    println!("Created default config file at {config_path_display}");
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

fn subcommand_completions() -> Command {
    return Command::new("completions")
        .about("Generates shell completions.")
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

fn subcommand_config() -> Command {
    return Command::new("config")
        .about("Configuration file options.")
        .subcommand(
            Command::new("create").about("Saves the default config file to the configuration file path. This command will fail if the file exists already.")
        )
        .subcommand(
            Command::new("default").about("Outputs the default configuration file to stdout.")
        )
        .subcommand(
            Command::new("path").about("Returns the default path for the configuration file.")
        );
}

fn subcommand_debug() -> Command {
    let mut cmd = Command::new("debug");
    cmd = cmd.about("Debug helpers for Oatmeal")
        .hide(true)
        .subcommand(
            Command::new("syntaxes").about("List all supported code highlighting languages.")
        )
        .subcommand(
            Command::new("resolve-syntax")
                .about("Resolves a string to a given highlighting syntax")
                .arg(
                    clap::Arg::new("entry")
                        .short('s')
                        .long("entry")
                        .help("Entry to resolve")
                        .required(true),
                )
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

    return cmd;
}

fn subcommand_sessions_delete() -> Command {
    return Command::new("delete")
        .about("Delete one or all sessions.")
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
                .help("Delete all sessions.")
                .num_args(0),
        )
        .group(
            ArgGroup::new("delete-args")
                .args(["session-id", "all"])
                .required(true),
        );
}

fn arg_backend() -> Arg {
    return Arg::new(ConfigKey::Backend.to_string())
        .short('b')
        .long(ConfigKey::Backend.to_string())
        .env("OATMEAL_BACKEND")
        .num_args(1)
        .help(format!(
            "The initial backend hosting a model to connect to. [default: {}]",
            Config::default(ConfigKey::Backend)
        ))
        .value_parser(PossibleValuesParser::new(BackendName::VARIANTS));
}

fn arg_backend_health_check_timeout() -> Arg {
    return Arg::new(ConfigKey::BackendHealthCheckTimeout.to_string())
        .long(ConfigKey::BackendHealthCheckTimeout.to_string())
        .env("OATMEAL_BACKEND_HEALTH_CHECK_TIMEOUT")
        .num_args(1)
        .help(
            format!("Time to wait in milliseconds before timing out when doing a healthcheck for a backend. [default: {}]", Config::default(ConfigKey::BackendHealthCheckTimeout)),
        );
}

fn arg_model() -> Arg {
    return Arg::new(ConfigKey::Model.to_string())
        .short('m')
        .long(ConfigKey::Model.to_string())
        .env("OATMEAL_MODEL")
        .num_args(1)
        .help("The initial model on a backend to consume. Defaults to the first model available from the backend if not set.");
}

fn subcommand_chat() -> Command {
    return Command::new("chat")
        .about("Start a new chat session.")
        .arg(arg_backend())
        .arg(arg_backend_health_check_timeout())
        .arg(arg_model());
}

fn subcommand_sessions() -> Command {
    return Command::new("sessions")
        .about("Manage past chat sessions.")
        .arg_required_else_help(true)
        .subcommand(Command::new("dir").about("Print the sessions cache directory path."))
        .subcommand(Command::new("list").about("List all previous sessions with their ids and models."))
        .subcommand(
            Command::new("open")
                .about("Open a previous session by ID. Omit passing any session ID to load an interactive selection.")
                .arg(
                    clap::Arg::new(ConfigKey::SessionID.to_string())
                        .short('i')
                        .long("id")
                        .help("Session ID")
                        .required(false),
                ),
        )
        .subcommand(subcommand_sessions_delete());
}

pub fn build() -> Command {
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
        .subcommand(subcommand_config())
        .subcommand(subcommand_debug())
        .subcommand(Command::new("manpages").about("Generates manpages and outputs to stdout."))
        .subcommand(subcommand_sessions())
        .arg(arg_backend())
        .arg(arg_backend_health_check_timeout())
        .arg(arg_model())
        .arg(
            Arg::new(ConfigKey::ConfigFile.to_string())
                .short('c')
                .long(ConfigKey::ConfigFile.to_string())
                .env("OATMEAL_CONFIG_FILE")
                .num_args(1)
                .help(format!("Path to configuration file [default: {}]", Config::default(ConfigKey::ConfigFile)))
                .global(true)
        )
        .arg(
            Arg::new(ConfigKey::Editor.to_string())
                .short('e')
                .long(ConfigKey::Editor.to_string())
                .env("OATMEAL_EDITOR")
                .num_args(1)
                .help(format!("The editor to integrate with. [default: {}]", Config::default(ConfigKey::Editor)))
                .value_parser(PossibleValuesParser::new(EditorName::VARIANTS))
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::Theme.to_string())
                .short('t')
                .long(ConfigKey::Theme.to_string())
                .env("OATMEAL_THEME")
                .num_args(1)
                .help(format!("Sets code syntax highlighting theme. [default: {}]", Config::default(ConfigKey::Theme)))
                .value_parser(PossibleValuesParser::new(themes))
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::ThemeFile.to_string())
                .long(ConfigKey::ThemeFile.to_string())
                .env("OATMEAL_THEME_FILE")
                .num_args(1)
                .help(
                    "Absolute path to a TextMate tmTheme to use for code syntax highlighting."
                )
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::LangChainURL.to_string())
                .long(ConfigKey::LangChainURL.to_string())
                .env("OATMEAL_LANGCHAIN_URL")
                .num_args(1)
                .help(format!("LangChain Serve API URL when using the LangChain backend. [default: {}]", Config::default(ConfigKey::LangChainURL)))
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::OllamaURL.to_string())
                .long(ConfigKey::OllamaURL.to_string())
                .env("OATMEAL_OLLAMA_URL")
                .num_args(1)
                .help(format!("Ollama API URL when using the Ollama backend. [default: {}]", Config::default(ConfigKey::OllamaURL)))
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::OpenAiURL.to_string())
                .long(ConfigKey::OpenAiURL.to_string())
                .env("OATMEAL_OPENAI_URL")
                .num_args(1)
                .help(format!("OpenAI API URL when using the OpenAI backend. Can be swapped to a compatible proxy. [default: {}]", Config::default(ConfigKey::OpenAiURL)))
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::OpenAiToken.to_string())
                .long(ConfigKey::OpenAiToken.to_string())
                .env("OATMEAL_OPENAI_TOKEN")
                .num_args(1)
                .help("OpenAI API token when using the OpenAI backend.")
                .global(true),
        )
        .arg(
            Arg::new(ConfigKey::ClaudeToken.to_string())
                .long(ConfigKey::ClaudeToken.to_string())
                .env("OATMEAL_CLAUDE_TOKEN")
                .num_args(1)
                .help("Anthropic's Claude API token when using the Claude backend.")
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
                }
                Some(("resolve-syntax", rs_matches)) => {
                    let entry = rs_matches.get_one::<String>("entry").unwrap();
                    let res = Syntaxes::get(entry);
                    println!("{:?}", res);
                }
                Some(("themes", _)) => {
                    println!("{}", Themes::list().join("\n"));
                }
                Some(("log-path", _)) => {
                    let log_path = dirs::cache_dir().unwrap().join("oatmeal/debug.log");
                    println!("{}", log_path.to_str().unwrap());
                }
                Some(("enum-config", _)) => {
                    let res = ConfigKey::VARIANTS.join("\n");
                    println!("{}", res);
                }
                _ => {
                    subcommand_debug().print_long_help()?;
                }
            }

            return Ok(false);
        }
        Some(("chat", subcmd_matches)) => {
            Config::load(build(), vec![&matches, subcmd_matches]).await?;
        }
        Some(("completions", subcmd_matches)) => {
            if let Some(completions) = subcmd_matches.get_one::<Shell>("shell").copied() {
                let mut app = build();
                print_completions(completions, &mut app);
            }
        }
        Some(("config", subcmd_matches)) => match subcmd_matches.subcommand() {
            Some(("create", _)) => {
                create_config_file().await?;
                return Ok(false);
            }
            Some(("default", _)) => {
                println!("{}", Config::serialize_default(build()));
                return Ok(false);
            }
            Some(("path", _)) => {
                println!("{}", Config::default(ConfigKey::ConfigFile));
                return Ok(false);
            }
            _ => {
                subcommand_config().print_long_help()?;
                return Ok(false);
            }
        },
        Some(("manpages", _)) => {
            clap_mangen::Man::new(build()).render(&mut io::stdout())?;
            return Ok(false);
        }
        Some(("sessions", subcmd_matches)) => match subcmd_matches.subcommand() {
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
                Config::load(build(), vec![&matches, open_matches]).await?;
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
        },
        _ => {
            Config::load(build(), vec![&matches]).await?;
        }
    }

    return Ok(true);
}
