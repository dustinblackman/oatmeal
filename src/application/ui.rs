use std::io;

use anyhow::Result;
use crossterm::cursor;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::Terminal;
use tokio::sync::mpsc;
use tui_textarea::Input;
use tui_textarea::Key;

use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::models::AcceptType;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::BackendPrompt;
use crate::domain::models::Loading;
use crate::domain::models::Message;
use crate::domain::models::SlashCommand;
use crate::domain::models::TextArea;
use crate::domain::services::AppState;
use crate::infrastructure::editors::EditorManager;

async fn start_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState<'_>,
    tx: mpsc::UnboundedSender<Action>,
    rx: &mut mpsc::UnboundedReceiver<Action>,
) -> Result<()> {
    let mut textarea = TextArea::default();
    let loading = Loading::default();

    #[cfg(feature = "dev")]
    {
        let test_str = "Write a function in Java that prints from 0 to 10. Return in markdown, add language to code blocks, describe the example before and after.";
        for char in test_str.chars() {
            textarea.input(Input {
                key: Key::Char(char),
                ctrl: false,
                alt: false,
            });
        }
    }

    loop {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Min(1), Constraint::Max(4)])
                .split(frame.size());

            if layout[0].width != app_state.last_known_width
                || layout[0].height != app_state.last_known_height
            {
                app_state.set_rect(layout[0]);
            }

            app_state
                .bubble_list
                .render(frame, layout[0], app_state.scroll.position);
            frame.render_stateful_widget(
                Scrollbar::new(ScrollbarOrientation::VerticalRight),
                layout[0].inner(&Margin {
                    vertical: 1,
                    horizontal: 0,
                }),
                &mut app_state.scroll.scrollbar_state,
            );

            if app_state.waiting_for_backend {
                loading.render(frame, layout[1]);
            } else {
                frame.render_widget(textarea.widget(), layout[1]);
            }
        })?;

        if app_state.waiting_for_backend {
            let event = rx.recv().await;
            if event.is_none() {
                continue;
            }

            match event.unwrap() {
                Action::BackendResponse(msg) => {
                    app_state.handle_backend_response(msg);
                }
                Action::MessageEvent(msg) => {
                    app_state.add_message(msg);
                    app_state.waiting_for_backend = false;
                }
                _ => (),
            }

            continue;
        }

        match crossterm::event::read()?.into() {
            Input { key: Key::Down, .. } => {
                app_state.scroll.down();
            }
            Input { key: Key::Up, .. } => {
                app_state.scroll.up();
            }
            Input {
                key: Key::Char('d'),
                ctrl: true,
                ..
            } => {
                app_state.scroll.down_page();
            }
            Input {
                key: Key::Char('u'),
                ctrl: true,
                ..
            } => {
                app_state.scroll.up_page();
            }
            Input {
                key: Key::Char('c'),
                ctrl: true,
                ..
            } => {
                break;
            }
            Input {
                key: Key::Enter, ..
            } => {
                let input_str = &textarea.lines().join("\n");
                if input_str.is_empty() {
                    continue;
                }

                let msg = Message::new(Author::User, input_str);
                textarea = TextArea::default();
                app_state.add_message(msg);

                if let Some(command) = SlashCommand::parse(input_str) {
                    if command.is_quit() {
                        break;
                    }

                    if command.is_accept_code_block() || command.is_replace_code_block() {
                        let mut accept_type = AcceptType::Append;
                        if command.is_replace_code_block() {
                            accept_type = AcceptType::Replace;
                        }

                        tx.send(Action::AcceptCodeBlock(
                            app_state.editor_context.clone(),
                            app_state.codeblocks.blocks_from_slash_commands(&command),
                            accept_type,
                        ))?;

                        continue;
                    }

                    if command.is_copy() {
                        tx.send(Action::CopyMessages(app_state.messages.clone()))?;
                        app_state.waiting_for_backend = true;
                        continue;
                    }
                }

                app_state.waiting_for_backend = true;
                let mut prompt =
                    BackendPrompt::new(input_str.to_string(), app_state.backend_context.clone());

                let user_messages_length = app_state
                    .messages
                    .iter()
                    .filter(|m| {
                        return m.author == Author::User && SlashCommand::parse(&m.text).is_none();
                    })
                    .collect::<Vec<_>>()
                    .len();
                if user_messages_length == 1 {
                    prompt.append_system_prompt(&app_state.editor_context);
                }

                tx.send(Action::BackendRequest(prompt))?;
            }
            input => {
                textarea.input(input);
            }
        }
    }

    return Ok(());
}

pub fn destruct_terminal_for_panic() {
    disable_raw_mode().unwrap();
    crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture).unwrap();
    crossterm::execute!(io::stdout(), cursor::Show).unwrap();
}

pub async fn start(
    tx: mpsc::UnboundedSender<Action>,
    rx: &mut mpsc::UnboundedReceiver<Action>,
) -> Result<()> {
    let stdout = io::stdout();
    let mut stdout = stdout.lock();

    enable_raw_mode()?;
    crossterm::execute!(stdout, EnterAlternateScreen, EnableMouseCapture)?;
    let term_backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(term_backend)?;
    let editor_name = Config::get(ConfigKey::Editor);
    let mut app_state = AppState::new(
        &Config::get(ConfigKey::Backend),
        &editor_name,
        &Config::get(ConfigKey::Model),
        &Config::get(ConfigKey::Theme),
        &Config::get(ConfigKey::ThemeFile),
    )
    .await?;

    start_loop(&mut terminal, &mut app_state, tx, rx).await?;
    if !editor_name.is_empty() {
        EditorManager::get(&editor_name)?.clear_context().await?;
    }

    disable_raw_mode()?;
    crossterm::execute!(
        terminal.backend_mut(),
        LeaveAlternateScreen,
        DisableMouseCapture
    )?;
    terminal.show_cursor()?;

    return Ok(());
}
