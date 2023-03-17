#![cfg_attr(
    all(not(debug_assertions), target_os = "windows"),
    windows_subsystem = "windows"
)]

use chatgpt_backend::api;
use common::{ConversationId, Prompt};
use tauri::{CustomMenuItem, Manager, Menu, Submenu, WindowMenuEvent};

#[tauri::command]
async fn completion<'r>(
    id: ConversationId,
    messages: Vec<api::Message>,
    state: tauri::State<'r, api::ChatGPT>,
) -> Result<api::Message, String> {
    state.chat_completion(id, messages).await
}

#[tauri::command]
fn start_conversation<'r>(hint: Option<String>, state: tauri::State<'r, api::ChatGPT>) -> Result<ConversationId, String> {
    state.start_conversation(hint)
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

#[tauri::command]
fn bundled_prompts() -> Result<Vec<Prompt>, String> {
    use csv::StringRecordsIter;
    use itertools::Itertools;

    let file = include_bytes!("../prompts.csv");
    let mut rdr = csv::ReaderBuilder::new().from_reader(&file[..]);
    rdr.records()
        .map_ok(|r| Prompt {
            act: r.get(0).unwrap().to_owned(),
            content: r.get(1).unwrap().to_owned(),
        })
        .try_collect::<_, Vec<_>, _>()
        .map_err(|e| e.to_string())
}

fn build_menu() -> Menu {
    let chats = CustomMenuItem::new("id_chats", "Chat");
    let coding = CustomMenuItem::new("id_coding", "Coding");

    let sub = Menu::new().add_item(chats).add_item(coding);
    let chatgpt = Submenu::new("GPT", sub);
    Menu::os_default("chatgpt").add_submenu(chatgpt)
}

fn handle_menu_event(e: WindowMenuEvent) {
    eprintln!("menu: {}", e.menu_item_id());
    match e.menu_item_id() {
        "id_chats" => {}
        "id_coding" => {}
        _ => unreachable!(),
    }
}

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            //eprintln!("{:?}", app.config());
            let cfg = tauri::api::path::app_config_dir(app.config().as_ref()).unwrap();
            eprintln!("config path: {:?}", cfg);

            //let names = app.windows().keys().map(|s| s.clone()).collect::<Vec<_>>();
            //eprintln!("windows: {:?}", names);

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
            bundled_prompts,
        ])
        .menu(build_menu())
        .on_menu_event(handle_menu_event)
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
