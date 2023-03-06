#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

#[allow(unused)]
mod api {
    use std::sync::{Mutex, Arc};

    use lazy_static::lazy_static;
    use config::{Config, ConfigError, File, Environment};
    use rand::{Rng, SeedableRng, rngs::StdRng, distributions::{Uniform, Distribution}};

    static COMPLETION_MODEL: &str = "gpt-3.5-turbo";
    static CODING_MODEL: &str = "code-davinci-002";
    static CHAT_API_PATH: &str = "https://api.openai.com/v1/chat/completions";

    use serde::{Deserialize, Serialize};

    #[derive(Serialize, Deserialize, Debug, Clone)]
    pub struct Message {
        role: String,
        content: String,
    }

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

    #[derive(Debug)]
    pub struct ChatGPT {
        settings: Settings,
        rng: Arc<Mutex<StdRng>>,
        pub cli: reqwest::Client,
    }

    impl ChatGPT {
        pub fn new() -> Self {
            let settings = ChatGPT::load_settings().unwrap();
            eprintln!("{:?}", settings);
            ChatGPT {
                settings,
                rng: Arc::new(Mutex::new(StdRng::from_entropy())),
                cli: reqwest::Client::new(),
            }
        }

        fn load_settings() -> Result<Settings, ConfigError> {
            let cfg = Config::builder()
                .set_default("model", COMPLETION_MODEL)?
                .set_default("stream", false)?
                .set_default("api_key", "")?
                .add_source(File::with_name("chatgpt"))
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

        pub async fn chat_completion(&self, messages: Vec<Message>) -> Result<Message, String> {
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
            eprintln!("api_key({})", api_key);

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

            match resp.json::<Answer>().await {
                Ok(result) => Ok(result.choices[0].message.clone()),
                Err(err) => {
                    eprintln!("{:?}", err);
                    Err(err.to_string())
                }
            }
        }
    }
}

#[tauri::command]
async fn completion<'r>(messages: Vec<api::Message>, state: tauri::State<'r, api::ChatGPT>) -> Result<api::Message, String> {
    state.chat_completion(messages).await
}


fn main() {
    tauri::Builder::default()
        .manage(api::ChatGPT::new())
        .invoke_handler(tauri::generate_handler![completion])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
