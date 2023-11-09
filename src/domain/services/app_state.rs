use anyhow::Result;
use ratatui::prelude::Rect;
use tokio::sync::mpsc;

use super::BubbleList;
use super::CodeBlocks;
use super::Scroll;
use super::Themes;
use crate::domain::models::AcceptType;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::BackendResponse;
use crate::domain::models::EditorContext;
use crate::domain::models::Message;
use crate::domain::models::MessageType;
use crate::domain::models::SlashCommand;
use crate::infrastructure::backends::BackendManager;
use crate::infrastructure::editors::EditorManager;

#[cfg(test)]
#[path = "app_state_test.rs"]
mod tests;

pub struct AppState<'a> {
    pub backend_context: String,
    pub bubble_list: BubbleList<'a>,
    pub codeblocks: CodeBlocks,
    pub editor_context: Option<EditorContext>,
    pub last_known_height: u16,
    pub last_known_width: u16,
    pub messages: Vec<Message>,
    pub scroll: Scroll,
    pub waiting_for_backend: bool,
}

impl<'a> AppState<'a> {
    pub async fn new(
        backend_name: &str,
        editor_name: &str,
        model_name: &str,
        theme_name: &str,
        theme_file: &str,
    ) -> Result<AppState<'a>> {
        let theme = Themes::load(theme_name, theme_file)?;

        let mut app_state = AppState {
            messages: vec![],
            bubble_list: BubbleList::new(theme),
            codeblocks: CodeBlocks::default(),
            backend_context: "".to_string(),
            waiting_for_backend: false,
            scroll: Scroll::default(),
            editor_context: None,
            last_known_width: 0,
            last_known_height: 0,
        };

        let backend = BackendManager::get(backend_name)?;
        let editor_res = EditorManager::get(editor_name);

        if let Ok(editor) = editor_res {
            let editor_context_op = editor.get_context().await?;
            if let Some(editor_context) = editor_context_op {
                let formatted = editor_context.format();
                app_state.editor_context = Some(editor_context);

                app_state.messages.push(Message::new(
                    Author::Model,
                    &format!(
                        "Hey there! Let's talk about the following: \n\n{}",
                        formatted
                    ),
                ));
            } else {
                app_state.messages.push(Message::new(
                    Author::Model,
                    "Hey there! What can I do for you?",
                ));
            }
        } else {
            app_state.messages.push(Message::new(
                Author::Model,
                "Hey there! What can I do for you?",
            ));
        }

        if let Err(err) = backend.health_check().await {
            app_state
                .messages
                .push(Message::new_with_type(
                    Author::Oatmeal,
                    MessageType::Error,
                    &format!("Hey, it looks like backend {backend_name} isn't running, I can't connect to it. You should double check that before we start talking, otherwise I may crash.\n\nError: {err}"),
                ));
        } else {
            let models = backend.list_models().await?;
            if !models.contains(&model_name.to_string()) {
                app_state
                .messages
                .push(Message::new_with_type(
                    Author::Oatmeal,
                    MessageType::Error,
                    format!("Model {model_name} doesn't exist for backend {backend_name}. You can use `/modellist` to view all avaiable models, and `/model NAME` to switch models.").as_str(),
                ));
            }
        }

        return Ok(app_state);
    }

    pub fn handle_backend_response(&mut self, msg: BackendResponse) {
        let last_message = self.messages.last_mut().unwrap();
        if last_message.author != Author::User {
            last_message.append(&msg.text);
        } else {
            self.messages.push(Message::new(msg.author, &msg.text));
        }

        self.sync_dependants();

        if msg.done {
            self.waiting_for_backend = false;
            if let Some(ctx) = msg.context {
                self.backend_context = ctx;
            }

            self.codeblocks.replace_from_messages(&self.messages);
        }
    }

    pub fn handle_slash_commands(
        &mut self,
        input_str: &str,
        tx: &mpsc::UnboundedSender<Action>,
    ) -> Result<(bool, bool)> {
        let mut should_break = false;
        let mut should_continue = false;

        if let Some(command) = SlashCommand::parse(input_str) {
            if command.is_quit() {
                should_break = true;
            }

            if command.is_append_code_block()
                || command.is_replace_code_block()
                || command.is_copy_code_block()
            {
                should_continue = true;

                let codeblocks_res = self.codeblocks.blocks_from_slash_commands(&command);
                if let Err(err) = codeblocks_res.as_ref() {
                    self.add_message(Message::new_with_type(
                        Author::Oatmeal,
                        MessageType::Error,
                        &format!(
                            "There was an error trying to parse your command:\n\n{:?}",
                            err
                        ),
                    ));

                    return Ok((should_break, should_continue));
                }

                if command.is_copy_code_block() {
                    tx.send(Action::CopyMessages(vec![Message::new(
                        Author::Model,
                        &codeblocks_res.unwrap(),
                    )]))?;
                    self.waiting_for_backend = true;
                    return Ok((should_break, should_continue));
                }

                let mut accept_type = AcceptType::Append;
                if command.is_replace_code_block() {
                    accept_type = AcceptType::Replace;
                }

                tx.send(Action::AcceptCodeBlock(
                    self.editor_context.clone(),
                    codeblocks_res.unwrap(),
                    accept_type,
                ))?;
            }

            if command.is_copy_chat() {
                tx.send(Action::CopyMessages(self.messages.clone()))?;
                self.waiting_for_backend = true;
            }
        }

        return Ok((should_break, should_continue));
    }

    pub fn set_rect(&mut self, rect: Rect) {
        self.last_known_width = rect.width;
        self.last_known_height = rect.height;
        self.sync_dependants();
    }

    pub fn add_message(&mut self, message: Message) {
        self.messages.push(message);
        self.sync_dependants();
        self.scroll.last();
    }

    fn sync_dependants(&mut self) {
        self.bubble_list
            .set_messages(&self.messages, self.last_known_width);

        self.scroll
            .set_state(self.bubble_list.len() as u16, self.last_known_height);

        if self.waiting_for_backend {
            self.scroll.last();
        }
    }
}
