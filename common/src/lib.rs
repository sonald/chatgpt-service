use serde::{Deserialize, Serialize};
use uuid::Uuid;

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Prompt {
    pub act: String,
    pub content: String,
}

impl Message {
    pub fn new_user(content: String) -> Self {
        Message {
            role: <KnownRoles as Into<&str>>::into(KnownRoles::User).into(),
            content,
        }
    }

    pub fn new_system(content: String) -> Self {
        Message {
            role: <KnownRoles as Into<&str>>::into(KnownRoles::System).into(),
            content,
        }
    }

    pub fn new_assistant(content: String) -> Self {
        Message {
            role: <KnownRoles as Into<&str>>::into(KnownRoles::Assistant).into(),
            content,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ConversationId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: ConversationId,
    pub title: String,
    pub messages: Vec<Message>,
}

#[derive(Debug, Clone, Copy)]
pub enum KnownRoles {
    System,
    User,
    Assistant,
}

use std::fmt::Display;

impl Display for KnownRoles {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        write!(
            f,
            "{}",
            match self {
                Self::System => "system",
                Self::User => "user",
                Self::Assistant => "assistant",
            }
        )
    }
}

impl From<&str> for KnownRoles {
    fn from(value: &str) -> Self {
        match value {
            "system" => Self::System,
            "user" => Self::User,
            "assistant" => Self::Assistant,
            _ => unreachable!(),
        }
    }
}

impl From<KnownRoles> for &str {
    fn from(value: KnownRoles) -> Self {
        match value {
            KnownRoles::System => "system",
            KnownRoles::User => "user",
            KnownRoles::Assistant => "assistant",
        }
    }
}
