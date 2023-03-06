use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize)]
pub struct Message {
    pub role: String,
    pub content: String,
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
        write!(f, "{}", match self {
            Self::System => "system",
            Self::User => "user",
            Self::Assistant => "assistant",
        })
    }
}

impl From<&str> for KnownRoles {
    fn from(value: &str) -> Self {
        match value {
            "system" => Self::System,
            "user" => Self::User,
            "assistant"=> Self::Assistant,
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
