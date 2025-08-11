#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use async_openai::{
    types::{CreateChatCompletionRequestArgs, ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs, ChatCompletionRequestAssistantMessageArgs},
    Client,
    config::OpenAIConfig
};
use tauri_plugin_store::{StoreBuilder, Store};
use tauri::{Manager, State, Wry};
use std::sync::Mutex;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatSession {
    messages: Vec<ChatMessage>,
    max_tokens: usize,
    max_messages: usize,
}

impl ChatSession {
    fn new(max_tokens: usize, max_messages: usize) -> Self {
        Self {
            messages: Vec::new(),
            max_tokens,
            max_messages,
        }
    }

    fn add_message(&mut self, role: String, content: String) {
        let message = ChatMessage {
            role,
            content,
            timestamp: std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        };
        
        self.messages.push(message);
        
        // Apply sliding window: keep only the most recent messages
        if self.messages.len() > self.max_messages {
            // Keep system message if it exists, then keep recent messages
            let system_messages: Vec<_> = self.messages.iter()
                .filter(|msg| msg.role == "system")
                .cloned()
                .collect();
            
            let recent_messages: Vec<_> = self.messages.iter()
                .filter(|msg| msg.role != "system")
                .rev()
                .take(self.max_messages - system_messages.len())
                .cloned()
                .collect();
            
            self.messages = system_messages;
            self.messages.extend(recent_messages.into_iter().rev());
        }
    }

    fn get_openai_messages(&self) -> Vec<async_openai::types::ChatCompletionRequestMessage> {
        self.messages.iter().map(|msg| {
            match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(&msg.content)
                    .build()
                    .unwrap()
                    .into(),
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(&msg.content)
                    .build()
                    .unwrap()
                    .into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
            }
        }).collect()
    }

    fn estimate_tokens(&self) -> usize {
        // Simple token estimation: ~4 characters per token
        self.messages.iter()
            .map(|msg| msg.content.len() / 4)
            .sum()
    }
}

#[tauri::command]
async fn send_message_to_openai(message: String, state: State<'_, AppState>) -> Result<String, String> {
    let api_key = {
        let store = state.store.lock().unwrap();
        match store.get("api_key") {
            Some(key) => key.as_str().unwrap().to_string(),
            None => return Err("API key not found".to_string()),
        }
    };

    let config = OpenAIConfig::new().with_api_key(api_key);
    let client = Client::with_config(config);

    // Get or create chat session
    let mut session = {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.entry("default".to_string())
            .or_insert_with(|| ChatSession::new(4000, 20))
            .clone()
    };

    // Add user message to session
    session.add_message("user".to_string(), message.clone());

    // Add system message if session is empty
    if session.messages.len() == 1 {
        session.add_message("system".to_string(), 
            "You are a helpful AI assistant. You can help with various tasks and provide responses in different formats like JSON, Markdown, XML, HTML, or plain text. Always respond in a helpful and informative way.".to_string());
    }

    // Check token limit and trim if necessary
    while session.estimate_tokens() > session.max_tokens && session.messages.len() > 2 {
        // Remove oldest non-system message
        if let Some(index) = session.messages.iter()
            .enumerate()
            .find(|(_, msg)| msg.role != "system")
            .map(|(i, _)| i) {
            session.messages.remove(index);
        } else {
            break;
        }
    }

    // Update session in state
    {
        let mut sessions = state.sessions.lock().unwrap();
        sessions.insert("default".to_string(), session.clone());
    }

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-3.5-turbo")
        .messages(session.get_openai_messages())
        .max_tokens(1000u16)
        .temperature(0.7)
        .build().map_err(|e| e.to_string())?;

    let response = client.chat().create(request).await.map_err(|e| e.to_string())?;

    let assistant_message = response.choices.get(0)
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_else(|| "No response received".to_string());

    // Add assistant response to session
    {
        let mut sessions = state.sessions.lock().unwrap();
        if let Some(session) = sessions.get_mut("default") {
            session.add_message("assistant".to_string(), assistant_message.clone());
        }
    }

    Ok(assistant_message)
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

#[tauri::command]
async fn clear_chat_history(state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    sessions.clear();
    Ok(())
}

#[tauri::command]
async fn get_chat_context(state: State<'_, AppState>) -> Result<Vec<ChatMessage>, String> {
    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get("default") {
        Ok(session.messages.clone())
    } else {
        Ok(Vec::new())
    }
}

struct AppState {
    store: Mutex<Store<Wry>>,
    sessions: Mutex<std::collections::HashMap<String, ChatSession>>,
}

fn main() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let store = StoreBuilder::new(app.handle(), ".settings.dat".parse().unwrap()).build();
            let sessions = std::collections::HashMap::new();
            app.manage(AppState { 
                store: Mutex::new(store),
                sessions: Mutex::new(sessions),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message_to_openai, 
            save_api_key, 
            load_api_key,
            clear_chat_history,
            get_chat_context
        ])
        .run(context)
        .expect("error while running tauri application");
}