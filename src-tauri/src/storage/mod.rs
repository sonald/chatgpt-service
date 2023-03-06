
#[derive(Debug, Clone, Copy)]
pub struct ConversationId(usize);

use common::Message;

pub trait Storage {
    fn start_conversation(&self, ctx: Option<String>) -> ConversationId;
    // append single chat
    fn chat(&self, id: ConversationId, question: &str, answer: &str);
    // replace whole conversation
    fn store_conversation(&self, id: ConversationId, msgs: Vec<Message>) -> Result<(), String>;
    fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String>;
}

use dashmap::DashMap;
use dashmap::mapref::one::Ref;

#[cfg(feature = "local-storage")]
struct LocalKVStorage {
    data: DashMap<ConversationId, Vec<Message>>,
}

impl Storage for LocalKVStorage {
    fn start_conversation(&self, ctx: Option<String>) -> ConversationId {
        todo!()
    }

    fn chat(&self, id: ConversationId, question: &str, answer: &str) {
        todo!()
    }

    fn store_conversation(&self, id: ConversationId, msgs: Vec<Message>) -> Result<(), String> {
        todo!()
    }

    fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String> {
        todo!()
    }
}
