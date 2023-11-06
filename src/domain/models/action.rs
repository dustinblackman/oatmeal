use super::AcceptType;
use super::BackendPrompt;
use super::BackendResponse;
use super::EditorContext;
use super::Message;

pub enum Action {
    AcceptCodeBlock(Option<EditorContext>, String, AcceptType),
    BackendRequest(BackendPrompt),
    BackendResponse(BackendResponse),
    CopyMessages(Vec<Message>),
    MessageEvent(Message),
}
