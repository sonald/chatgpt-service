#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use chatgpt_backend::api;
use common::ConversationId;

#[tauri::command]
async fn completion<'r>(
    id: ConversationId,
    messages: Vec<api::Message>,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<api::Message, String> {
    state.chat_completion(id, messages).await
}

#[tauri::command]
fn start_conversation<'r>(state: tauri::State<'r, api::ChatGPT>) -> Result<ConversationId, String> {
    state.start_conversation()
}

#[tauri::command]
fn get_conversations<'r>(
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<Vec<ConversationId>, String> {
    state.get_conversations()
}

#[tauri::command]
fn get_conversation<'r>(
    id: ConversationId,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<Vec<api::Message>, String> {
    state.get_conversation(id)
}

#[tauri::command]
fn get_title<'r>(
    id: ConversationId,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<String, String> {
    state.get_title(id)
}

#[tauri::command]
fn set_title<'r>(
    id: ConversationId,
    title: String,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<(), String> {
    state.set_title(id, title)
}

#[tauri::command]
async fn suggest_title<'r>(
    id: ConversationId,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<String, String> {
    state.suggest_title(id).await
}

use tauri::Manager;
fn main() {
    tauri::Builder::default()
        .setup(|app| {
            //eprintln!("{:?}", app.config());
            let cfg = tauri::api::path::app_config_dir(app.config().as_ref()).unwrap();
            eprintln!("config path: {:?}", cfg);

            app.handle().manage(api::ChatGPT::new(cfg));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            completion,
            start_conversation,
            get_conversations,
            get_conversation,
            get_title,
            set_title,
            suggest_title,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
