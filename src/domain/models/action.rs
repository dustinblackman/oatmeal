use super::AcceptType;
use super::BackendPrompt;
use super::EditorContext;
use super::Message;

pub enum Action {
    AcceptCodeBlock(Option<EditorContext>, String, AcceptType),
    BackendRequest(BackendPrompt),
    CopyMessages(Vec<Message>),
}
