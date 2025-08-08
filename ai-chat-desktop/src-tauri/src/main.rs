#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use async_openai::{
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs},
    Client,
    config::OpenAIConfig
};
use tauri_plugin_store::{StoreBuilder, Store};
use tauri::{Manager, State, Wry};
use std::sync::Mutex;

#[tauri::command]
async fn send_message_to_openai(message: String, state: State<'_, AppState>) -> Result<String, String> {
    let store = state.store.lock().unwrap();
    let api_key = match store.get("api_key") {
        Some(key) => key.as_str().unwrap().to_string(),
        None => return Err("API key not found".to_string()),
    };

    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-3.5-turbo")
        .messages([
            ChatCompletionRequestSystemMessageArgs::default()
                .content("You are a helpful assistant.")
                .build().unwrap().into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(message)
                .build().unwrap().into(),
        ])
        .build().map_err(|e| e.to_string())?;

    let response = client.chat().create(request).await.map_err(|e| e.to_string())?;

    Ok(response.choices.get(0).unwrap().message.content.clone().unwrap())
}

#[tauri::command]
async fn save_api_key(api_key: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut store = state.store.lock().unwrap();
    store.insert("api_key".to_string(), api_key.into()).map_err(|e| e.to_string())?;
    store.save().map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
async fn load_api_key(state: State<'_, AppState>) -> Result<Option<String>, String> {
    let store = state.store.lock().unwrap();
    Ok(store.get("api_key").map(|v| v.as_str().unwrap().to_string()))
}

struct AppState {
    store: Mutex<Store<Wry>>,
}

fn main() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let store = StoreBuilder::new(app.handle(), ".settings.dat".parse().unwrap()).build();
            app.manage(AppState { store: Mutex::new(store) });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![send_message_to_openai, save_api_key, load_api_key])
        .run(context)
        .expect("error while running tauri application");
}