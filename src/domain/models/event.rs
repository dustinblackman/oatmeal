use tokio::sync::mpsc::UnboundedSender;
use tui_textarea::Input;

use super::BackendResponse;
use super::Message;

pub enum Event {
    BackendMessage(Message),
    BackendPromptResponse(BackendResponse),
    EditPrompt(UnboundedSender<Event>),
    EditPromptMessage(Message),
    NewPrompt(String),
    KeyboardCharInput(Input),
    KeyboardCTRLC(),
    KeyboardCTRLE(),
    KeyboardCTRLO(),
    KeyboardCTRLR(),
    KeyboardEnter(),
    KeyboardPaste(String),
    UITick(),
    UIScrollDown(),
    UIScrollUp(),
    UIScrollPageDown(),
    UIScrollPageUp(),
}
