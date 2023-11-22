use anyhow::bail;
use anyhow::Result;
use test_utils::codeblock_fixture;
use test_utils::insta_snapshot;
use tokio::sync::mpsc;

use super::AppState;
use crate::domain::models::AcceptType;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::BackendResponse;
use crate::domain::models::Message;
use crate::domain::models::MessageType;
use crate::domain::services::BubbleList;
use crate::domain::services::CodeBlocks;
use crate::domain::services::Scroll;
use crate::domain::services::Themes;

impl Default for AppState<'static> {
    fn default() -> AppState<'static> {
        let theme = Themes::load("base16-onedark", "").unwrap();
        return AppState {
            backend_context: "".to_string(),
            bubble_list: BubbleList::new(theme),
            codeblocks: CodeBlocks::default(),
            editor_context: None,
            exit_warning: false,
            last_known_height: 300,
            last_known_width: 100,
            messages: vec![],
            session_id: "test".to_string(),
            scroll: Scroll::default(),
            waiting_for_backend: false,
        };
    }
}

mod handle_slash_commands {
    use super::*;

    #[test]
    fn it_breaks_on_quit() -> Result<()> {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        let (should_break, should_continue) = app_state.handle_slash_commands("/q", &tx)?;

        assert!(should_break);
        assert!(!should_continue);
        assert!(!app_state.waiting_for_backend);

        return Ok(());
    }

    #[test]
    fn it_appends_code_block() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        app_state
            .codeblocks
            .replace_from_messages(&[Message::new(Author::Model, codeblock_fixture())]);

        let (should_break, should_continue) = app_state.handle_slash_commands("/append 1", &tx)?;

        assert!(!should_break);
        assert!(should_continue);
        assert!(!app_state.waiting_for_backend);

        let event = rx.blocking_recv().unwrap();
        match event {
            Action::AcceptCodeBlock(_context, codeblock, accept_type) => {
                assert_eq!(accept_type, AcceptType::Append);
                insta_snapshot(|| {
                    insta::assert_toml_snapshot!(codeblock);
                })
            }
            _ => bail!("Wrong enum"),
        }

        return Ok(());
    }

    #[test]
    fn it_replaces_code_block() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        app_state
            .codeblocks
            .replace_from_messages(&[Message::new(Author::Model, codeblock_fixture())]);

        let (should_break, should_continue) = app_state.handle_slash_commands("/replace 1", &tx)?;

        assert!(!should_break);
        assert!(should_continue);
        assert!(!app_state.waiting_for_backend);

        let event = rx.blocking_recv().unwrap();
        match event {
            Action::AcceptCodeBlock(_context, codeblock, accept_type) => {
                assert_eq!(accept_type, AcceptType::Replace);
                insta_snapshot(|| {
                    insta::assert_toml_snapshot!(codeblock);
                })
            }
            _ => bail!("Wrong enum"),
        }

        return Ok(());
    }

    #[test]
    fn it_copies_code_block() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        app_state
            .codeblocks
            .replace_from_messages(&[Message::new(Author::Model, codeblock_fixture())]);

        let (should_break, should_continue) = app_state.handle_slash_commands("/copy 1", &tx)?;

        assert!(!should_break);
        assert!(should_continue);
        assert!(app_state.waiting_for_backend);

        let event = rx.blocking_recv().unwrap();
        match event {
            Action::CopyMessages(messages) => {
                assert_eq!(messages[0].author, Author::Model);
                insta_snapshot(|| {
                    insta::assert_toml_snapshot!(messages[0].text);
                })
            }
            _ => bail!("Wrong enum"),
        }

        return Ok(());
    }

    #[test]
    fn it_copies_chat() -> Result<()> {
        let (tx, mut rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        app_state.add_message(Message::new(Author::User, "Hello world"));

        let (should_break, should_continue) = app_state.handle_slash_commands("/copy", &tx)?;

        assert!(!should_break);
        assert!(!should_continue);
        assert!(app_state.waiting_for_backend);

        let event = rx.blocking_recv().unwrap();
        match event {
            Action::CopyMessages(messages) => {
                assert_eq!(messages.len(), 1)
            }
            _ => bail!("Wrong enum"),
        }

        return Ok(());
    }

    #[test]
    fn it_returns_error_message_on_invalid_codeblock() -> Result<()> {
        let (tx, _rx) = mpsc::unbounded_channel::<Action>();
        let mut app_state = AppState::default();
        app_state
            .codeblocks
            .replace_from_messages(&[Message::new(Author::Model, codeblock_fixture())]);

        let (should_break, should_continue) =
            app_state.handle_slash_commands("/replace 1000", &tx)?;
        let last_message = app_state.messages.last().unwrap();

        assert!(!should_break);
        assert!(should_continue);
        assert!(!app_state.waiting_for_backend);
        assert_eq!(last_message.author, Author::Oatmeal);
        assert_eq!(last_message.message_type(), MessageType::Error);
        insta::assert_snapshot!(last_message.text, @r###"
        There was an error trying to parse your command:

        999 is out of bounds.
        "###);

        return Ok(());
    }
}

mod handle_backend_response {
    use super::*;

    #[test]
    fn it_handles_new_backend_response() {
        let mut app_state = AppState::default();
        app_state
            .messages
            .push(Message::new(Author::User, "Do something for me!"));
        let backend_response = BackendResponse {
            author: Author::Model,
            text: "All done!".to_string(),
            done: true,
            context: Some("icanrememberthingsnow".to_string()),
        };
        app_state.handle_backend_response(backend_response);

        assert_eq!(app_state.messages.len(), 2);
    }

    #[test]
    fn it_handles_bad_backend_response() {
        let mut app_state = AppState::default();
        app_state
            .messages
            .push(Message::new(Author::User, "Do something for me!"));
        let backend_response = BackendResponse {
            author: Author::Model,
            text: "All done!".to_string(),
            done: true,
            context: Some("".to_string()),
        };
        app_state.handle_backend_response(backend_response);

        assert_eq!(app_state.messages.len(), 3);
        assert_eq!(
            app_state.messages.last().unwrap().message_type(),
            MessageType::Error
        );
    }
}
