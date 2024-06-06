use tokio::sync::mpsc;

use super::*;
use crate::configuration::{Config, ConfigKey};
use crate::domain::models::Event;
use crate::domain::models::{Author, Message, MessageType};

#[test]
fn it_can_build_buildable() {
    assert!(matches!(
        ActiveService::build(),
        TypestateService {
            state: Buildable {}
        }
    ));
}

#[test]
fn it_can_make_startable() {
    let (tx, _rx) = mpsc::unbounded_channel::<Event>();
    assert!(matches!(
        ActiveService::build().event_tx(&tx),
        TypestateService {
            state: Startable { .. }
        },
    ));
}

#[test]
fn it_can_create_temp_file() {
    let (tx, _rx) = mpsc::unbounded_channel::<Event>();
    let svc = ActiveService::build()
        .event_tx(&tx)
        .create_temp_file()
        .unwrap();
    let path = svc.state.temp_file.path();
    assert!(std::fs::read_to_string(path).is_ok());
}

#[test]
fn it_creates_tempfile_with_prompt() {
    let (tx, _rx) = mpsc::unbounded_channel::<Event>();
    let prompt = "a multiline
    string";
    let svc = ActiveService::build()
        .event_tx(&tx)
        .prompt(prompt)
        .create_temp_file()
        .unwrap();
    let contents = std::fs::read_to_string(svc.state.temp_file.path()).unwrap();
    assert!(contents.starts_with(prompt));
}

#[test]
fn it_creates_tempfile_with_messages() {
    let username = Config::get(ConfigKey::Username);
    let model = Config::get(ConfigKey::Model);
    Config::set(ConfigKey::Username, "bugs bunny");
    Config::set(ConfigKey::Model, "chatgpt84");

    let (tx, _rx) = mpsc::unbounded_channel::<Event>();
    let messages = [
        Message::new(Author::Model, "as an ai language model..."),
        Message::new(Author::User, "what's up doc"),
        Message::new_with_type(Author::Oatmeal, MessageType::Error, "oatmeal error"),
    ];
    let svc = ActiveService::build()
        .event_tx(&tx)
        .messages(&messages)
        .create_temp_file()
        .unwrap();
    let contents = std::fs::read_to_string(svc.state.temp_file.path()).unwrap();
    let expected_content_end = "\n\
        Oatmeal:\n\
        oatmeal error\n\
        \n\
        bugs bunny:\n\
        what's up doc\n\
        \n\
        chatgpt84:\n\
        as an ai language model...\n\
    ";
    assert!(contents.ends_with(expected_content_end));

    Config::set(ConfigKey::Username, &username);
    Config::set(ConfigKey::Model, &model);
}

#[test]
fn it_always_adds_one_empty_line() {
    let (tx, _rx) = mpsc::unbounded_channel::<Event>();
    let svc = ActiveService::build()
        .event_tx(&tx)
        .create_temp_file()
        .unwrap();
    let contents = std::fs::read_to_string(svc.state.temp_file.path()).unwrap();
    assert_eq!(
        contents
            .lines()
            .map(|line| return line.is_empty())
            .take(3)
            .collect::<Vec<bool>>(),
        vec![true, false, false]
    );
}

#[test]
fn it_can_parse_a_prompt_file() {
    let (tx, mut rx) = mpsc::unbounded_channel::<Event>();
    let mut svc = ActiveService::build()
        .event_tx(&tx)
        .create_temp_file()
        .unwrap();

    let contents = std::fs::read_to_string(svc.state.temp_file.path()).unwrap();
    let prompt = "new prompt\n\
        on multiple\n\
        \n\
        lines\n\
    ";
    let new_contents = format!("{prompt}{contents}");
    svc.state.temp_file.rewind().unwrap();
    svc.state
        .temp_file
        .write_all(new_contents.as_bytes())
        .unwrap();

    let svc = TypestateService {
        state: Parseable {
            event_tx: tx,
            temp_file: svc.state.temp_file,
            original_prompt: "".to_owned(),
        },
    };
    let _svc = svc.parse().unwrap();
    let msg = rx.blocking_recv().unwrap();
    assert!(matches!(msg, Event::NewPrompt(_)));
    let Event::NewPrompt(new_prompt) = msg else {
        panic!();
    };
    assert_eq!(prompt, new_prompt);
}
