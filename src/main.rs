#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod application;
mod configuration;
mod domain;
mod infrastructure;

use std::env;
use std::process;

use anyhow::Error;
use domain::models::Action;
use domain::models::BackendName;
use domain::models::Event;
use domain::services::clipboard::ClipboardService;
use infrastructure::backends::BackendManager;
use tokio::sync::mpsc;
use tokio::task;
use yansi::Paint;

use crate::application::cli;
use crate::application::ui;
use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::services::actions::ActionsService;

#[cfg(feature = "dhat-heap")]
#[global_allocator]
static ALLOC: dhat::Alloc = dhat::Alloc;

fn handle_error(err: Error) {
    eprintln!(
            "{}",
            Paint::red(format!(
                "Oh no! Oatmeal has failed with the following app version and error.\n\nVersion: {}\nCommit: {}\nError: {}",
                env!("CARGO_PKG_VERSION"),
                env!("VERGEN_GIT_DESCRIBE"),
                err
            ))
        );

    let backtrace = err.backtrace();
    if backtrace.to_string() == "disabled backtrace" {
        let args = env::args().collect::<Vec<String>>().join(" ");
        eprintln!(
            "\nIf you could spare a moment, please head over to the docs to report this issue! It contains steps to assist in debugging."
        );
        eprintln!(
            "\nhttps://github.com/dustinblackman/oatmeal/blob/v{}/README.md#report-an-issue",
            env!("CARGO_PKG_VERSION")
        );
        eprintln!("\nOtherwise, running the following can help explain further what the issue is:");
        eprintln!("\nRUST_BACKTRACE=1 {args}");
    } else {
        eprintln!("\n{}", backtrace);
    }

    process::exit(1);
}

#[tokio::main]
async fn main() {
    std::panic::set_hook(Box::new(|panic_info| {
        ui::destruct_terminal_for_panic();
        better_panic::Settings::auto().create_panic_handler()(panic_info);
    }));

    #[cfg(feature = "dhat-heap")]
    let _profiler = dhat::Profiler::new_heap();

    let debug_log_dir = env::var("OATMEAL_LOG_DIR").unwrap_or_else(|_| {
        return dirs::cache_dir()
            .unwrap()
            .join("oatmeal")
            .to_string_lossy()
            .to_string();
    });

    let file_appender = tracing_appender::rolling::never(debug_log_dir, "debug.log");
    let (writer, _guard) = tracing_appender::non_blocking(file_appender);
    if env::var("RUST_LOG")
        .unwrap_or_else(|_| return "".to_string())
        .contains("oatmeal")
    {
        tracing_subscriber::fmt()
            .json()
            .with_max_level(tracing::Level::DEBUG)
            .with_writer(writer)
            .init();
    }

    let ready_res = cli::parse().await;
    if let Err(ready_err) = ready_res {
        handle_error(ready_err);
        return;
    }
    if !ready_res.unwrap() {
        process::exit(0);
    }

    let (action_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();
    let (event_tx, event_rx) = mpsc::unbounded_channel::<Event>();

    let mut background_futures = task::JoinSet::new();
    background_futures.spawn(async move {
        let backend = BackendName::parse(Config::get(ConfigKey::Backend)).unwrap();
        return ActionsService::start(
            BackendManager::get(backend).unwrap(),
            event_tx,
            &mut action_rx,
        )
        .await;
    });

    if let Err(clipboard_err) = ClipboardService::healthcheck() {
        tracing::warn!(err = ?clipboard_err, "Clipboard service is unable to start")
    } else {
        background_futures.spawn(async move {
            return ClipboardService::start().await;
        });
    }

    let ui_future = ui::start(action_tx, event_rx);

    let res = tokio::select!(
        res = background_futures.join_next() => res.unwrap().unwrap(),
        res = ui_future => res,
    );

    if res.is_err() {
        ui::destruct_terminal_for_panic();
        handle_error(res.unwrap_err());
    }

    #[cfg(feature = "dhat-heap")]
    drop(_profiler);

    process::exit(0);
}
