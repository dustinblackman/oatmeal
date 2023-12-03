use std::io;

use anyhow::Result;
use crossterm::cursor;
use crossterm::event::DisableMouseCapture;
use crossterm::event::EnableMouseCapture;
use crossterm::terminal::disable_raw_mode;
use crossterm::terminal::enable_raw_mode;
use crossterm::terminal::is_raw_mode_enabled;
use crossterm::terminal::EnterAlternateScreen;
use crossterm::terminal::LeaveAlternateScreen;
use ratatui::backend::CrosstermBackend;
use ratatui::prelude::*;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::Terminal;
use tokio::sync::mpsc;

use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::BackendPrompt;
use crate::domain::models::Event;
use crate::domain::models::Loading;
use crate::domain::models::Message;
use crate::domain::models::SlashCommand;
use crate::domain::models::TextArea;
use crate::domain::services::events::EventsService;
use crate::domain::services::AppState;
use crate::infrastructure::editors::EditorManager;

async fn start_loop<B: Backend>(
    terminal: &mut Terminal<B>,
    app_state: &mut AppState<'_>,
    tx: mpsc::UnboundedSender<Action>,
    rx: mpsc::UnboundedReceiver<Event>,
) -> Result<()> {
    let mut events = EventsService::new(rx);
    let mut textarea = TextArea::default();
    let loading = Loading::default();

    #[cfg(feature = "dev")]
    {
        use tui_textarea::Input;
        use tui_textarea::Key;

        let test_str = "Write a function in Java that prints from 0 to 10. Return in markdown, add language to code blocks, describe the example before and after.";
        for char in test_str.chars() {
            textarea.input(Input {
                key: Key::Char(char),
                ctrl: false,
                alt: false,
                shift: false,
            });
        }
    }

    loop {
        terminal.draw(|frame| {
            let layout = Layout::default()
                .direction(Direction::Vertical)
                .constraints(vec![Constraint::Min(1), Constraint::Max(4)])
                .split(frame.size());

            if layout[0].width as usize != app_state.last_known_width
                || layout[0].height as usize != app_state.last_known_height
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

        macro_rules! send_user_message {
            ( $input_str:expr ) => {
                let input_str = $input_str;

                let msg = Message::new(Author::User, &input_str);
                textarea = TextArea::default();
                app_state.add_message(msg);

                let (should_break, should_continue) =
                    app_state.handle_slash_commands(input_str, &tx)?;
                if should_break {
                    break;
                }
                if should_continue {
                    continue;
                }

                app_state.waiting_for_backend = true;
                let mut prompt =
                    BackendPrompt::new(input_str.to_string(), app_state.backend_context.clone());

                if app_state.backend_context.is_empty() && SlashCommand::parse(&input_str).is_none()
                {
                    prompt.append_system_prompt(&app_state.editor_context);
                }

                tx.send(Action::BackendRequest(prompt))?;
                app_state.save_session().await?;
            };
        }

        match events.next().await? {
            Event::BackendMessage(msg) => {
                app_state.add_message(msg);
                app_state.waiting_for_backend = false;
            }
            Event::BackendPromptResponse(msg) => {
                app_state.handle_backend_response(msg.clone());
                if msg.done {
                    app_state.save_session().await?;
                }
            }
            Event::KeyboardCharInput(input) => {
                if app_state.waiting_for_backend {
                    continue;
                }

                // Windows submits a null event right after CTRL+C. Ignore it.
                if input.key != tui_textarea::Key::Null {
                    app_state.exit_warning = false;
                }

                textarea.input(input);
            }
            Event::KeyboardCTRLC() => {
                if app_state.waiting_for_backend {
                    app_state.waiting_for_backend = false;
                    tx.send(Action::BackendAbort())?;
                } else if !app_state.exit_warning {
                    app_state.add_message(Message::new(
                        Author::Oatmeal,
                        "If you wish to quit, hit CTRL+C one more time, or use /quit",
                    ));
                    app_state.exit_warning = true;
                } else {
                    break;
                }
            }
            Event::KeyboardCTRLR() => {
                let last_message = app_state
                    .messages
                    .iter()
                    .filter(|msg| {
                        return msg.author == Author::User
                            && SlashCommand::parse(&msg.text).is_none();
                    })
                    .last();
                if let Some(message) = last_message.cloned() {
                    send_user_message!(&message.text);
                }
            }
            Event::KeyboardEnter() => {
                if app_state.waiting_for_backend {
                    continue;
                }
                let input_str = &textarea.lines().join("\n");
                if input_str.is_empty() {
                    continue;
                }
                send_user_message!(input_str);
            }
            Event::UIResize() => {
                continue;
            }
            Event::UIScrollDown() => {
                app_state.scroll.down();
            }
            Event::UIScrollUp() => {
                app_state.scroll.up();
            }
            Event::UIScrollPageDown() => {
                app_state.scroll.down_page();
            }
            Event::UIScrollPageUp() => {
                app_state.scroll.up_page();
            }
        }
    }

    return Ok(());
}

pub fn destruct_terminal_for_panic() {
    if let Ok(enabled) = is_raw_mode_enabled() {
        if enabled {
            let _ = disable_raw_mode();
            let _ = crossterm::execute!(io::stdout(), LeaveAlternateScreen, DisableMouseCapture);
            let _ = crossterm::execute!(io::stdout(), cursor::Show);
        }
    }
}

pub async fn start(
    tx: mpsc::UnboundedSender<Action>,
    rx: mpsc::UnboundedReceiver<Event>,
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
        &Config::get(ConfigKey::SessionID),
    )
    .await?;

    start_loop(&mut terminal, &mut app_state, tx, rx).await?;
    if !editor_name.is_empty() {
        let editor = EditorManager::get(&editor_name)?;
        if editor.health_check().await.is_ok() {
            editor.clear_context().await?;
        }
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
