use anyhow::Result;
use tokio::sync::mpsc;

use super::clipboard::ClipboardService;
use crate::config::Config;
use crate::config::ConfigKey;
use crate::domain::models::AcceptType;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::EditorContext;
use crate::domain::models::Event;
use crate::domain::models::Message;
use crate::domain::models::MessageType;
use crate::domain::models::SlashCommand;
use crate::infrastructure::backends::BackendBox;
use crate::infrastructure::backends::BackendManager;
use crate::infrastructure::editors::EditorManager;

pub fn help_text() -> String {
    let text = r#"
COMMANDS:
- /modelist (/ml) - Lists all available models from the backend.
- /model (/model) [MODEL_NAME] - Sets the specified model as the active model.
- /append (/a) [CODE_BLOCK_NUMBER?] - Appends code blocks to an editor. See Code Actions for more details.
- /replace (/r) [CODE_BLOCK_NUMBER?] - Replaces selections with code blocks in an editor. See Code Actions for more details.
- /copy (/c) [CODE_BLOCK_NUMBER?] - Copies the entire chat history to your clipboard. When a CODE_BLOCK_NUMBER is used, only the specified copy blocks are copied to clipboard. See Code Actions for more details.
- /quit (/q) - Exit Oatmeal.
- /help (/h) - Provides this help menu.

HOTKEYS:
- Up arrow - Scroll up
- Down arrow - Scroll down
- CTRL+U - Page up
- CTRL+D - Page down

CODE ACTIONS:
When working with models that provide code, and using an editor integration, Oatmeal has the capabilities to read selected code from an editor, and submit model provided code back in to an editor. Each code block provided by a model is indexed with a (NUMBER) at the beginning of the block to make it easily identifiable.

- /append (/a) [CODE_BLOCK_NUMBER?] will append one-to-many model provided code blocks to the open file in your editior.
- /replace (/r) [CODE_BLOCK_NUMBER?] - will replace selected code with one-to-many model provided code blocks to the open file in your editor.
- /copy (/c) [CODE_BLOCK_NUMBER?] - will append one-to-many model provided code blocks to your clipboard, no matter the editor integration being used.

The CODE_BLOCK_NUMBER allows you to select several code blocks to send back to your editor at once. The parameter can be set as follows:
- `1` - Selects the first code block
- `1,3,5` - Selects code blocks 1, 3, and 5.
- `2..5`- Selects an inclusive range of code blocks between 2 and 5.
- None - Selects the last provided code block.
        "#;

    return text.trim().to_string();
}

async fn model_list(backend: &BackendBox, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    let res = backend
        .list_models()
        .await?
        .iter()
        .map(|model| {
            return format!("- {model}");
        })
        .collect::<Vec<String>>();

    tx.send(Event::BackendMessage(Message::new(
        Author::Oatmeal,
        res.join("\n").as_str(),
    )))?;

    return Ok(());
}

async fn model_set(
    backend: &BackendBox,
    tx: &mpsc::UnboundedSender<Event>,
    text: &str,
) -> Result<()> {
    let model_name = text.split(' ').last().unwrap().to_string();
    if SlashCommand::parse(&model_name).is_some() {
        let msg = Message::new_with_type(
            Author::Oatmeal,
            MessageType::Error,
            "You must specify a model name with `/model` or `/m`. Run `/help` more details.",
        );
        tx.send(Event::BackendMessage(msg))?;
        return Ok(());
    }

    let models = backend.list_models().await?;

    if !models.contains(&model_name) {
        let backend_name = Config::get(ConfigKey::Backend);
        let msg = Message::new_with_type(
            Author::Oatmeal,
            MessageType::Error,
            &format!(
                "No model named {model_name} found in backend {backend_name}. Did you mistype it?"
            ),
        );
        tx.send(Event::BackendMessage(msg))?;
        return Ok(());
    }

    Config::set(ConfigKey::Model, &model_name);

    tx.send(Event::BackendMessage(Message::new(
        Author::Model,
        &format!("{model_name} has entered the chat."),
    )))?;

    return Ok(());
}

async fn accept_codeblock(
    context: Option<EditorContext>,
    codeblock: String,
    accept_type: AcceptType,
) -> Result<()> {
    let editor_name = Config::get(ConfigKey::Editor);
    let editor = EditorManager::get(&editor_name)?;

    if editor_name == "clipboard" {
        editor
            .send_codeblock(EditorContext::default(), codeblock, accept_type)
            .await?;

        return Ok(());
    }

    if let Some(editor_context) = context {
        editor
            .send_codeblock(editor_context, codeblock, accept_type)
            .await?;
    }

    return Ok(());
}

fn copy_messages(messages: Vec<Message>, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    if messages.len() == 1 {
        ClipboardService::set(messages[0].text.to_string())?;
    } else {
        let formatted = messages
            .iter()
            .map(|message| {
                return format!("{}: {}", message.author_formatted, message.text);
            })
            .collect::<Vec<String>>()
            .join("\n\n");

        ClipboardService::set(formatted)?;
    }

    tx.send(Event::BackendMessage(Message::new(
        Author::Oatmeal,
        "Copied chat log to clipboard.",
    )))?;

    return Ok(());
}

fn help(tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    tx.send(Event::BackendMessage(Message::new(
        Author::Oatmeal,
        &help_text(),
    )))?;

    return Ok(());
}

pub struct ActionsService {}

impl ActionsService {
    pub async fn start(
        tx: mpsc::UnboundedSender<Event>,
        rx: &mut mpsc::UnboundedReceiver<Action>,
    ) -> Result<()> {
        let backend = BackendManager::get(&Config::get(ConfigKey::Backend))?;

        loop {
            let event = rx.recv().await;
            if event.is_none() {
                continue;
            }

            match event.unwrap() {
                Action::AcceptCodeBlock(context, codeblock, accept_type) => {
                    accept_codeblock(context, codeblock, accept_type).await?;
                }
                Action::CopyMessages(messages) => {
                    copy_messages(messages, &tx)?;
                }
                Action::BackendRequest(prompt) => {
                    if let Some(command) = SlashCommand::parse(&prompt.text) {
                        if command.is_model_list() {
                            model_list(&backend, &tx).await?;
                            continue;
                        }
                        if command.is_model_set() {
                            model_set(&backend, &tx, &prompt.text).await?;
                            continue;
                        }
                        if command.is_help() {
                            help(&tx)?;
                            continue;
                        }
                    }

                    backend.get_completion(prompt, &tx).await?;
                }
            }
        }
    }
}
