#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

mod api {
    use lazy_static::lazy_static;

    lazy_static! {
        static ref OPENAI_API_KEY: String = std::env::var("OPENAI_API_KEY").unwrap_or("".to_string());
    }

    //static MODEL: &str = "text-davinci-003";
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
    pub struct Params<'a> {
        model: &'a str,
        temperature: f32,
        stream: bool,
        messages: Vec<Message>,
    }

    #[derive(Debug, Default, Deserialize)]
    struct Usage {
        prompt_tokens: usize,
        completion_tokens: usize,
        total_tokens: usize,
    }

    #[derive(Debug, Deserialize)]
    struct Choice {
        index: usize,
        message: Message,
        finish_reason: String,
    }

    #[derive(Debug, Default, Deserialize)]
    struct Answer {
        id: String,
        object: String,
        choices: Vec<Choice>,
        usage: Usage
    }

    pub async fn chat_completion(messages: Vec<Message>) -> Message {
        let data = Params {
            model: COMPLETION_MODEL,
            temperature: 1.0,
            messages,
            stream: false,
        };

        eprintln!("completion({})", serde_json::to_string(&data).unwrap());

        let cli = reqwest::Client::new();
        let resp = cli
            .post(CHAT_API_PATH)
            .header(
                reqwest::header::AUTHORIZATION,
                format!("Bearer {}", OPENAI_API_KEY.as_str()),
            ).header(
            reqwest::header::CONTENT_TYPE,
            "application/json",
            )
                .json(&data)
                .send()
                .await
                .unwrap();

        match resp.json::<Answer>().await {
            Ok(result) => result.choices[0].message.clone(),
            Err(err) => {
                eprintln!("{:?}", err);
                Message {
                    role: "assistant".to_string(),
                    content: "".to_string(),
                }
            }
        }
    }
}

#[tauri::command]
async fn completion(messages: Vec<api::Message>) -> api::Message {
    api::chat_completion(messages).await
}


fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![completion])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
