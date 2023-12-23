use std::sync::Arc;

use anyhow::Result;
use tokio::sync::mpsc;
use tokio::task::JoinHandle;

use super::clipboard::ClipboardService;
use crate::configuration::Config;
use crate::configuration::ConfigKey;
use crate::domain::models::AcceptType;
use crate::domain::models::Action;
use crate::domain::models::Author;
use crate::domain::models::BackendBox;
use crate::domain::models::BackendPrompt;
use crate::domain::models::EditorContext;
use crate::domain::models::EditorName;
use crate::domain::models::Event;
use crate::domain::models::Message;
use crate::domain::models::MessageType;
use crate::domain::models::SlashCommand;
use crate::infrastructure::editors::EditorManager;

pub fn help_text() -> String {
    let text = r#"
COMMANDS:
- /modellist (/ml) - Lists all available models from the backend.
- /model (/model) [MODEL_NAME,MODEL_INDEX] - Sets the specified model as the active model. You can pass either the model name, or the index from `/modellist`.
- /append (/a) [CODE_BLOCK_NUMBER?] - Appends code blocks to an editor. See Code Actions for more details.
- /replace (/r) [CODE_BLOCK_NUMBER?] - Replaces selections with code blocks in an editor. See Code Actions for more details.
- /copy (/c) [CODE_BLOCK_NUMBER?] - Copies the entire chat history to your clipboard. When a `CODE_BLOCK_NUMBER` is used, only the specified copy blocks are copied to clipboard. See Code Actions for more details.
- /quit /exit (/q) - Exit Oatmeal.
- /help (/h) - Provides this help menu.

HOTKEYS:
- Up arrow - Scroll up.
- Down arrow - Scroll down.
- CTRL+U - Page up.
- CTRL+D - Page down.
- CTRL+C - Interrupt waiting for prompt response if in progress, otherwise exit.
- CTRL+R - Resubmit your last message to the backend.

CODE ACTIONS:
When working with models that provide code, and using an editor integration, Oatmeal has the capabilities to read selected code from an editor, and submit model provided code back in to an editor. Each code block provided by a model is indexed with a (NUMBER) at the beginning of the block to make it easily identifiable.

- /append (/a) [CODE_BLOCK_NUMBER?] will append one-to-many model provided code blocks to the open file in your editor.
- /replace (/r) [CODE_BLOCK_NUMBER?] - will replace selected code in your editor with one-to-many model provided code blocks.
- /copy (/c) [CODE_BLOCK_NUMBER?] - Copies the entire chat history to your clipboard. When a `CODE_BLOCK_NUMBER` is used it will append one-to-many model provided code blocks to your clipboard, no matter the editor integration.

The `CODE_BLOCK_NUMBER` allows you to select several code blocks to send back to your editor at once. The parameter can be set as follows:
- `1` - Selects the first code block
- `1,3,5` - Selects code blocks 1, 3, and 5.
- `2..5`- Selects an inclusive range of code blocks between 2 and 5.
- None - Selects the last provided code block.
        "#;

    return text.trim().to_string();
}

async fn model_list(backend: &BackendBox, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    let mut models = backend.list_models().await?;
    models.sort();

    let res = models
        .iter()
        .enumerate()
        .map(|(idx, model)| {
            let n = idx + 1;
            return format!("- ({n}) {model}");
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
    let mut model_name = text.split(' ').last().unwrap().to_string();
    if SlashCommand::parse(&model_name).is_some() {
        let msg = Message::new_with_type(
            Author::Oatmeal,
            MessageType::Error,
            "You must specify a model name with `/model` or `/m`. Run `/help` more details.",
        );
        tx.send(Event::BackendMessage(msg))?;
        return Ok(());
    }

    let mut models = backend.list_models().await?;
    models.sort();

    if let Ok(idx) = model_name.parse::<usize>() {
        if idx < 1 || idx > models.len() {
            let msg = Message::new_with_type(
                Author::Oatmeal,
                MessageType::Error,
                &format!("{idx} is not a valid index from the model list."),
            );
            tx.send(Event::BackendMessage(msg))?;
            return Ok(());
        }
        model_name = models[idx - 1].to_string();
    }

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
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<()> {
    let editor_name = EditorName::parse(Config::get(ConfigKey::Editor)).unwrap();
    let editor = EditorManager::get(editor_name.clone())?;
    let mut context_mut = context;

    if editor_name == EditorName::Clipboard || editor_name == EditorName::None {
        context_mut = Some(EditorContext::default());
    }

    if let Some(editor_context) = context_mut {
        let res = editor
            .send_codeblock(editor_context, codeblock, accept_type)
            .await;

        if let Err(err) = res {
            tx.send(Event::BackendMessage(Message::new_with_type(
                Author::Oatmeal,
                MessageType::Error,
                &format!("Failed to commuicate with editor:\n\n{err}"),
            )))?;
        }
    }

    if editor_name == EditorName::Clipboard {
        tx.send(Event::BackendMessage(Message::new(
            Author::Oatmeal,
            "Copied codeblocks to clipboard.",
        )))?;
    }

    return Ok(());
}

fn copy_messages(messages: Vec<Message>, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    let mut payload = messages[0].text.to_string();
    if messages.len() > 1 {
        payload = messages
            .iter()
            .map(|message| {
                return format!("{}: {}", message.author.to_string(), message.text);
            })
            .collect::<Vec<String>>()
            .join("\n\n");
    }

    if let Err(err) = ClipboardService::set(payload) {
        tx.send(Event::BackendMessage(Message::new_with_type(
            Author::Oatmeal,
            MessageType::Error,
            &format!("Failed to copy to clipboard:\n\n{err}"),
        )))?;

        return Ok(());
    }
    tx.send(Event::BackendMessage(Message::new(
        Author::Oatmeal,
        "Copied chat log to clipboard.",
    )))?;

    return Ok(());
}

fn worker_error(err: anyhow::Error, tx: &mpsc::UnboundedSender<Event>) -> Result<()> {
    tx.send(Event::BackendMessage(Message::new_with_type(
        Author::Oatmeal,
        MessageType::Error,
        &format!("The backend failed with the following error: {:?}", err),
    )))?;

    return Ok(());
}

async fn completions(
    backend: &BackendBox,
    prompt: BackendPrompt,
    tx: &mpsc::UnboundedSender<Event>,
) -> Result<()> {
    if Config::get(ConfigKey::Model).is_empty() {
        let models = backend.list_models().await?;
        Config::set(ConfigKey::Model, &models[0]);
    }

    backend.get_completion(prompt, tx).await?;

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
        backend: BackendBox,
        tx: mpsc::UnboundedSender<Event>,
        rx: &mut mpsc::UnboundedReceiver<Action>,
    ) -> Result<()> {
        let backend_arc = Arc::new(backend);

        // Lazy default.
        let mut worker: JoinHandle<Result<()>> = tokio::spawn(async {
            return Ok(());
        });

        loop {
            let event = rx.recv().await;
            if event.is_none() {
                continue;
            }

            let worker_tx = tx.clone();
            match event.unwrap() {
                Action::AcceptCodeBlock(context, codeblock, accept_type) => {
                    accept_codeblock(context, codeblock, accept_type, &tx).await?;
                }
                Action::CopyMessages(messages) => {
                    copy_messages(messages, &tx)?;
                }
                Action::BackendAbort() => {
                    worker.abort();
                }
                Action::BackendRequest(prompt) => {
                    if let Some(command) = SlashCommand::parse(&prompt.text) {
                        if command.is_model_list() {
                            model_list(&backend_arc, &tx).await?;
                            continue;
                        }
                        if command.is_model_set() {
                            model_set(&backend_arc, &tx, &prompt.text).await?;
                            continue;
                        }
                        if command.is_help() {
                            help(&tx)?;
                            continue;
                        }
                    }

                    let backend_worker = backend_arc.clone();
                    worker = tokio::spawn(async move {
                        if let Err(err) = completions(&backend_worker, prompt, &worker_tx).await {
                            worker_error(err, &worker_tx)?;
                        }
                        return Ok(());
                    });
                }
            }
        }
    }
}
