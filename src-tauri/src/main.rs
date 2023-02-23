#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use lazy_static::lazy_static;

lazy_static! {
    static ref OPENAI_API_KEY: String =
        { std::env::var("OPENAI_API_KEY").unwrap_or("".to_string()) };
}

static MODEL: &str = "text-davinci-003";
static API_PATH: &str = "https://api.openai.com/v1/completions";

use serde::{Deserialize, Serialize};

#[derive(Serialize)]
struct Params {
    model: String,
    temperature: f32,
    max_tokens: usize,
    prompt: String,
}

///```json
///{
///     "id":"cmpl-6jPmzCoNwIH3Rx5EHrGhQPVHxPQdX",
///     "object":"text_completion",
///     "created":1676281913,
///     "model":"text-davinci-003",
///     "choices":[{"text":"\n\nQ: What did the fish say when he hit a concrete wall?\nA: Dam!","index":0,"logprobs":null,"finish_reason":"stop"}],
///     "usage":{"prompt_tokens":4,"completion_tokens":21,"total_tokens":25}}
///```
#[derive(Debug, Default, Deserialize)]
struct Choice {
    text: String,
    index: usize,
    finish_reason: String,
}

#[derive(Debug, Default, Deserialize)]
struct Answer {
    model: String,
    object: String,
    choices: Vec<Choice>,
}

#[tauri::command]
async fn completion(prompt: String) -> String {
    eprintln!("completion({})", prompt);

    let data = Params {
        model: MODEL.to_owned(),
        temperature: 1.0,
        max_tokens: 1000,
        prompt,
    };

    let cli = reqwest::Client::new();
    let resp = cli
        .post(API_PATH)
        .header(
            reqwest::header::AUTHORIZATION,
            format!("Bearer {}", OPENAI_API_KEY.as_str()),
        )
        .json(&data)
        .send()
        .await
        .unwrap();

    match resp.json::<Answer>().await {
        Ok(result) => result.choices[0].text.clone(),
        Err(err) => {
            eprintln!("{:?}", err);
            "".to_string()
        }
    }
}

fn main() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![completion])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
