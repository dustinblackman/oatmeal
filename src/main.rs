#![deny(clippy::implicit_return)]
#![allow(clippy::needless_return)]

mod application;
mod config;
mod domain;
mod infrastructure;

use std::env;

use config::Config;
use config::ConfigKey;
use domain::models::Action;
use domain::services::clipboard::ClipboardService;
use tokio::sync::mpsc;

use crate::application::cli;
use crate::application::ui;
use crate::domain::services::actions::ActionsService;

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
    cli::parse();

    let (action_tx, mut ui_rx) = mpsc::unbounded_channel::<Action>();
    let (ui_tx, mut action_rx) = mpsc::unbounded_channel::<Action>();

    tokio::spawn(async move {
        ActionsService::start(action_tx, &mut action_rx)
            .await
            .unwrap();
    });
    tokio::spawn(async move {
        ClipboardService::start().await.unwrap();
    });

    ui::start(ui_tx, &mut ui_rx).await.unwrap();
}
