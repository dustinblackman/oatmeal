use super::AcceptType;
use super::BackendPrompt;
use super::EditorContext;
use super::Message;

pub enum Action {
    AcceptCodeBlock(Option<EditorContext>, String, AcceptType),
    BackendAbort(),
    BackendRequest(BackendPrompt),
    CopyMessages(Vec<Message>),
    EditPromptBegin(),
}
