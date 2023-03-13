#![allow(unused)]

use std::{sync::{Mutex, Arc}, path::{Path, PathBuf}};

use lazy_static::lazy_static;
use config::{Config, ConfigError, File, Environment};
use rand::{Rng, SeedableRng, rngs::StdRng, distributions::{Uniform, Distribution}};

static COMPLETION_MODEL: &str = "gpt-3.5-turbo";
static CODING_MODEL: &str = "code-davinci-002";
static CHAT_API_PATH: &str = "https://api.openai.com/v1/chat/completions";

use serde::{Deserialize, Serialize};
pub use common::{Message, ConversationId};

use crate::storage::Storage;
#[cfg(feature = "local-storage")]
use crate::storage::local::KVStorage as LocalStorage;
#[cfg(feature = "persist-storage")]
use crate::storage::disk::KVStorage as DiskStorage;

#[derive(Serialize, Debug)]
#[allow(unused)]
pub struct Params<'a> {
    model: &'a str,
    temperature: f32,
    stream: bool,
    messages: Vec<Message>,
}

#[derive(Debug, Default, Deserialize)]
#[allow(unused)]
struct Usage {
    prompt_tokens: usize,
    completion_tokens: usize,
    total_tokens: usize,
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
struct Choice {
    index: usize,
    message: Message,
    finish_reason: String,
}

#[derive(Debug, Default, Deserialize)]
#[allow(unused)]
struct Answer {
    id: String,
    object: String,
    choices: Vec<Choice>,
    usage: Usage
}

#[derive(Debug, Deserialize)]
#[allow(unused)]
pub struct Settings {
    model: String,
    temperature: f32,
    stream: bool,
    api_key: String,
    api_keys: Vec<String>,
}

pub struct ChatGPT {
    settings: Settings,
    rng: Arc<Mutex<StdRng>>,
    pub cli: reqwest::Client,

    store: Box<dyn Storage + Send + Sync>,
}

impl ChatGPT {
    pub fn new<P: AsRef<Path>>(cfg_path: P) -> Self {
        let settings = ChatGPT::load_settings(cfg_path.as_ref()).unwrap();

        ChatGPT {
            settings,
            rng: Arc::new(Mutex::new(StdRng::from_entropy())),
            cli: reqwest::Client::new(),

            store: Self::get_store(cfg_path), 
        }
    }

    #[cfg(feature = "persist-storage")]
    fn get_store<P: AsRef<Path>>(cfg_path: P) -> Box<dyn Storage + Send + Sync> {
        Box::new(DiskStorage::new(cfg_path).unwrap())
    }

    #[cfg(all(feature = "local-storage", not(feature = "persist-storage")))]
    fn get_store<P: AsRef<Path>>(cfg_path: P) -> Box<dyn Storage + Send + Sync> {
        Box::new(LocalStorage::new())
    }

    fn load_settings<P: AsRef<Path>>(cfg_path: P) -> Result<Settings, ConfigError> {
        let mut fpath = PathBuf::from(cfg_path.as_ref());
        fpath.push("chatgpt");

        let cfg = Config::builder()
            .set_default("model", COMPLETION_MODEL)?
            .set_default("stream", false)?
            .set_default("api_key", "")?
            .add_source(File::with_name(fpath.as_path().to_str().unwrap()))
            .add_source(Environment::with_prefix("openai"))
            .build()?;

        cfg.try_deserialize()
    }

    fn pick_api_key(&self) -> Option<&str> {
        if self.settings.api_key.is_empty() {
            let range = Uniform::from(0..self.settings.api_keys.len());
            let i = self.rng.lock().unwrap().sample(range);
            self.settings.api_keys.get(i).map(|s| s.as_str())
        } else {
            Some(self.settings.api_key.as_str())
        }
    }


    pub fn start_conversation(&self) -> Result<ConversationId, String> {
        eprintln!("start_conversation");
        Ok(self.store.start_conversation(None))
    }

    pub fn get_conversations(&self) -> Result<Vec<ConversationId>, String> {
        self.store.get_conversations()
    }
    
    pub fn get_conversation(&self, id: ConversationId) -> Result<Vec<Message>, String> {
        self.store.get_conversation(id)
    }

    pub fn get_title(&self, id: ConversationId) -> Result<String, String> {
        self.store.get_title(id).ok_or("no title".to_string())
    }

    pub fn set_title(&self, id: ConversationId, title: String) -> Result<(), String> {
        self.store.store_title(id, title)
    }

    pub async fn suggest_title(&self, id: ConversationId) -> Result<String, String> {
        eprintln!("suggest_title({:?})", id);
        let msgs = self.store.get_conversation(id).and_then(|dialogue| {
            let dialogue = dialogue.into_iter().take(6).map(|msg| msg.content).collect::<Vec<_>>().join("\n");

            Ok(vec! {
                Message::new_system("Act as a summarizer and summarize this dialogue".to_string()),
                Message::new_user(dialogue),
            })
        });

        match msgs {
            Ok(msgs) => self.generate_completion(msgs).await.map(|msg| { msg.content }),
            Err(e) => Err(e),
        }
    }

    //FIXME: what if tokens in `messages` exceed 4096
    pub async fn generate_completion(&self, mut messages: Vec<Message>) -> Result<Message, String> {
        let data = Params {
            model: &self.settings.model,
            temperature: self.settings.temperature,
            messages,
            stream: self.settings.stream,
        };

        let api_key = match self.pick_api_key() {
            Some(key) => key,
            None => return Err("api key is not set".to_string()),
        };

        eprintln!("completion({})", serde_json::to_string(&data).unwrap());
        eprintln!("api_key({}...)", api_key.get(..8).unwrap_or(""));

        let mut retried = false;
        let resp = loop {
            match self.cli
                .post(CHAT_API_PATH)
                .header(
                    reqwest::header::AUTHORIZATION,
                    format!("Bearer {}", api_key),
                ).header(
                reqwest::header::CONTENT_TYPE,
                "application/json",
                )
                .json(&data)
                .send()
                .await {
                    Ok(resp) => break resp,
                    Err(e) => {
                        if retried {
                            return Err(format!("request error: {}", e));
                        }
                        eprintln!("retry on error: {:?}", e);
                        retried = true;
                    }
                }
        };

        let data = resp.bytes().await.unwrap();
        eprintln!("data: {}", String::from_utf8_lossy(&data));
        match serde_json::from_slice::<Answer>(&data) {
            Ok(result) => {
                Ok(result.choices[0].message.clone())
            },
            Err(err) => {
                eprintln!("answer: {:?}", err);
                Err(err.to_string())
            }
        }
    }


    pub async fn chat_completion(&self, id: ConversationId, mut messages: Vec<Message>) -> Result<Message, String> {
        self.generate_completion(messages.clone()).await.and_then(|msg| {
            messages.push(msg.clone());
            self.store.store_conversation(id, messages).unwrap();
            Ok(msg)
        })
    }
}


