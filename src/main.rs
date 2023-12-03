#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod application;
mod config;
mod domain;
mod infrastructure;

use std::env;
use std::process;

use anyhow::anyhow;
use anyhow::Error;
use anyhow::Result;
use config::Config;
use config::ConfigKey;
use domain::models::Action;
use domain::models::Event;
use domain::services::clipboard::ClipboardService;
use owo_colors::OwoColorize;
use tokio::sync::mpsc;
use tokio::task;

use crate::application::cli;
use crate::application::ui;
use crate::domain::services::actions::ActionsService;

async fn flatten<T>(handle: task::JoinHandle<Result<T>>) -> Result<()> {
    return match handle.await {
        Ok(Ok(_result)) => Ok(()),
        Ok(Err(err)) => Err(err),
        Err(err) => Err(anyhow!(format!("Failed flatten handle: {:?}", err))),
    };
}

fn handle_error(err: Error) {
    eprintln!(
            "{}",
            format!(
                "Oh no! Oatmeal has failed with the following app version and error.\n\nVersion: {}\nCommit: {}\nError: {}",
                env!("CARGO_PKG_VERSION"),
                env!("VERGEN_GIT_DESCRIBE"),
                err
            )
            .red()
        );

    let backtrace = err.backtrace();
    if backtrace.to_string() == "disabled backtrace" {
        let args = env::args().collect::<Vec<String>>().join(" ");
        eprintln!(
            "\nIf you could spare a moment, please head over to the docs to report this issue!"
        );
        eprintln!(
            "\nhttps://github.com/dustinblackman/oatmeal/blob/v{}/README.md#report-an-issue",
            env!("CARGO_PKG_VERSION")
        );
        eprintln!("\nAfterward debugging is setup, you can rerun your command with the following:");
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

    Config::set(
        ConfigKey::Username,
        &env::var("USER").unwrap_or_else(|_| return "User".to_string()),
    );
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

    let actions_future = tokio::spawn(async move {
        return ActionsService::start(event_tx, &mut action_rx).await;
    });
    let clipboard_future = tokio::spawn(async move {
        return ClipboardService::start().await;
    });
    let ui_future = ui::start(action_tx, event_rx);

    let res = tokio::select!(
        res = flatten(actions_future) => res,
        res = flatten(clipboard_future) => res,
        res = ui_future => res,
    );

    if res.is_err() {
        ui::destruct_terminal_for_panic();
        handle_error(res.unwrap_err());
    }

    process::exit(0);
}
