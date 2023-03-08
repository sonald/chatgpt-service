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

#[cfg(feature = "local-storage")]
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

#[cfg(feature = "persist-storage")]
pub mod disk {
    use std::path::Path;

    use super::*;

    use itertools::Itertools;
    use serde::{Deserialize, Serialize};
    use sled::{Config, Db, IVec};

    #[derive(Debug)]
    pub struct KVStorage {
        db: Db,
    }

    impl KVStorage {
        pub fn new<P: AsRef<Path>>(path: P) -> sled::Result<Self> {
            Ok(KVStorage {
                db: Config::new().temporary(false).path(path).open()?,
            })
        }
    }

    #[derive(Serialize, Deserialize)]
    struct MessageList(Vec<Message>);

    impl TryFrom<IVec> for MessageList {
        type Error = String;

        fn try_from(value: IVec) -> Result<Self, Self::Error> {
            serde_json::from_slice(value.as_ref()).map_err(|e| e.to_string())
        }
    }

    impl TryFrom<MessageList> for Vec<u8> {
        type Error = String;

        fn try_from(value: MessageList) -> Result<Self, Self::Error> {
            serde_json::to_vec(&value).map_err(|e| e.to_string())
        }
    }

    impl Storage for KVStorage {
        fn store_message(&self, id: ConversationId, msg: Message) {
            self.db
                .fetch_and_update(&id.0, |old| match old {
                    Some(old) => {
                        let mut msgs: MessageList = IVec::from(old).try_into().unwrap();
                        msgs.0.push(msg.clone());
                        Some::<Vec<u8>>(msgs.try_into().unwrap())
                    }
                    None => None,
                })
                .unwrap();
        }

        fn store_conversation(&self, id: ConversationId, msgs: Vec<Message>) -> Result<(), String> {
            let data: Vec<u8> = MessageList(msgs).try_into().unwrap();
            self.db
                .insert(&id.0, data)
                .map(|_| ())
                .map_err(|e| e.to_string())
        }

        fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String> {
            self.db
                .get(&id.0)
                .map_err(|e| e.to_string())
                .and_then(|old| match old {
                    Some(val) => val.try_into().map(|v: MessageList| v.0),
                    None => Err("".to_string()),
                })
        }

        fn get_conversations(&self) -> Result<Vec<ConversationId>, String> {
            self.db
                .scan_prefix(&[])
                .keys()
                .map_ok(|k| ConversationId(Uuid::from_slice(&k).unwrap()))
                .try_collect::<_, Vec<_>, _>()
                .map_err(|e| e.to_string())
        }
    }
}
