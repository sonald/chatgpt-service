use common::{ConversationId, Message};
use uuid::Uuid;

pub trait Storage {
    fn start_conversation(&self, ctx: Option<String>) -> ConversationId {
        let id = ConversationId(Uuid::new_v4());
        if let Some(ctx) = ctx {
            self.store_message(id, Message::new_system(ctx));
        }

        id
    }

    fn store_message(&self, id: ConversationId, msg: Message);
    // replace whole conversation
    fn store_conversation(&self, id: ConversationId, msgs: Vec<Message>) -> Result<(), String>;
    fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String>;
    fn get_conversations(&self) -> Result<Vec<ConversationId>, String>;
}

//#[cfg(feature = "local-storage")]
pub mod local {
    use super::*;

    use dashmap::DashMap;

    #[derive(Debug)]
    pub struct KVStorage {
        data: DashMap<ConversationId, Vec<Message>>,
    }

    impl KVStorage {
        pub fn new() -> Self {
            KVStorage {
                data: DashMap::new(),
            }
        }
    }

    impl Storage for KVStorage {
        fn store_message(&self, id: ConversationId, msg: Message) {
            let mut chats = self.data.entry(id).or_default();
            chats.push(msg);
        }

        fn store_conversation(&self, id: ConversationId, msgs: Vec<Message>) -> Result<(), String> {
            let mut chats = self.data.entry(id).or_default();
            let _ = std::mem::replace(chats.value_mut(), msgs);

            Ok(())
        }

        fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String> {
            self.data
                .get(&id)
                .map(|kv| kv.value().clone())
                .ok_or("no conversation found".to_string())
        }

        fn get_conversations(&self) -> Result<Vec<ConversationId>, String> {
            Ok(self.data.iter().map(|v| *v.key()).collect::<Vec<_>>())
        }
         
    }
}
