#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use chatgpt_backend::api;

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
