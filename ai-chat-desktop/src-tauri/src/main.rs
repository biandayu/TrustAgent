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
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatMessage {
    role: String,
    content: String,
    timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ChatSession {
    id: String,
    title: String,
    messages: Vec<ChatMessage>,
    created_at: u64,
    updated_at: u64,
}

impl ChatSession {
    fn new(id: String, title: String) -> Self {
        let now = now_ts();
        Self {
            id,
            title,
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }
}

fn now_ts() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

struct AppState {
    store: Mutex<Store<Wry>>,
    sessions: Mutex<HashMap<String, ChatSession>>,
    current_session_id: Mutex<Option<String>>,
}

fn get_sessions_dir() -> PathBuf {
    let mut dir = std::env::current_dir().unwrap();
    dir.push(".chats");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    dir
}

fn save_session_to_file(session: &ChatSession) -> Result<(), String> {
    let dir = get_sessions_dir();
    let path = dir.join(format!("{}.json", session.id));
    let content = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn load_sessions_from_files() -> HashMap<String, ChatSession> {
    let dir = get_sessions_dir();
    let mut map = HashMap::new();
    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            if let Ok(content) = fs::read_to_string(entry.path()) {
                if let Ok(session) = serde_json::from_str::<ChatSession>(&content) {
                    map.insert(session.id.clone(), session);
                }
            }
        }
    }
    map
}

fn generate_session_title(messages: &[ChatMessage]) -> String {
    // 简单策略：取首条用户消息的前20字符
    messages.iter()
        .find(|m| m.role == "user")
        .map(|m| {
            let mut t = m.content.trim().to_string();
            if t.len() > 20 { t.truncate(20); t.push_str("..."); }
            t
        })
        .unwrap_or_else(|| "New Chat".to_string())
}

#[tauri::command]
async fn new_chat_session(state: State<'_, AppState>) -> Result<String, String> {
    let mut sessions = state.sessions.lock().unwrap();
    let mut current_id = state.current_session_id.lock().unwrap();
    let id = Uuid::new_v4().to_string();
    let session = ChatSession::new(id.clone(), "New Chat".to_string());
    sessions.insert(id.clone(), session);
    *current_id = Some(id.clone());
    Ok(id)
}

#[tauri::command]
async fn switch_chat_session(session_id: String, state: State<'_, AppState>) -> Result<ChatSession, String> {
    let mut current_id = state.current_session_id.lock().unwrap();
    let sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get(&session_id) {
        *current_id = Some(session_id.clone());
        Ok(session.clone())
    } else {
        Err("Session not found".to_string())
    }
}

#[tauri::command]
async fn get_all_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>, String> {
    let sessions = state.sessions.lock().unwrap();
    let mut list: Vec<_> = sessions.values().cloned().collect();
    // 按更新时间倒序排序
    list.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    Ok(list)
}

#[tauri::command]
async fn save_current_session(state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    let current_id = state.current_session_id.lock().unwrap();
    if let Some(id) = &*current_id {
        if let Some(session) = sessions.get_mut(id) {
            // 如果标题是默认的“New Chat”并且会话里有消息了，就生成一个真实的标题
            if session.title == "New Chat" && !session.messages.is_empty() {
                session.title = generate_session_title(&session.messages);
            }
            session.updated_at = now_ts();
            save_session_to_file(session)?;
        }
    }
    Ok(())
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

    // --- Start of lock-holding block ---
    let openai_msgs = {
        let mut sessions = state.sessions.lock().unwrap();
        let mut current_id = state.current_session_id.lock().unwrap();
        let session_id = current_id.clone().unwrap_or_else(|| {
            let id = Uuid::new_v4().to_string();
            sessions.insert(id.clone(), ChatSession::new(id.clone(), "New Chat".to_string()));
            *current_id = Some(id.clone());
            id
        });
        let session = sessions.get_mut(&session_id).unwrap();

        // Add user message
        session.messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.clone(),
            timestamp: now_ts(),
        });
        session.updated_at = now_ts();

        // Add system message if none exists
        if session.messages.iter().all(|m| m.role != "system") {
            session.messages.insert(0, ChatMessage {
                role: "system".to_string(),
                content: "You are a helpful AI assistant. You can help with various tasks and provide responses in different formats like JSON, Markdown, XML, HTML, or plain text. Always respond in a helpful and informative way.".to_string(),
                timestamp: now_ts(),
            });
        }

        // Sliding window context management
        if session.messages.len() > 20 {
            let system = session.messages.iter().find(|m| m.role == "system").cloned();
            let mut recent: Vec<_> = session.messages.iter().filter(|m| m.role != "system").rev().take(19).cloned().collect();
            recent.reverse();
            session.messages = system.into_iter().chain(recent).collect();
        }

        // Construct request messages (cloned)
        session.messages.iter().map(|msg| {
            match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content.clone())
                    .build().unwrap().into(),
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build().unwrap().into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content.clone())
                    .build().unwrap().into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build().unwrap().into(),
            }
        }).collect::<Vec<_>>()
    }; // --- All locks are dropped here ---

    let request = CreateChatCompletionRequestArgs::default()
        .model("gpt-3.5-turbo")
        .messages(openai_msgs)
        .max_tokens(1000u16)
        .temperature(0.7)
        .build().map_err(|e| e.to_string())?;

    // --- Await is now safe ---
    let response = client.chat().create(request).await.map_err(|e| e.to_string())?;
    let assistant_message = response.choices.get(0)
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_else(|| "No response received".to_string());

    // --- Re-acquire locks to update state ---
    {
        let mut sessions = state.sessions.lock().unwrap();
        let current_id = state.current_session_id.lock().unwrap();
        if let Some(id) = &*current_id {
            if let Some(session) = sessions.get_mut(id) {
                session.messages.push(ChatMessage {
                    role: "assistant".to_string(),
                    content: assistant_message.clone(),
                    timestamp: now_ts(),
                });
                session.updated_at = now_ts();
            }
        }
    }

    Ok(assistant_message)
}

#[tauri::command]
async fn get_current_session(state: State<'_, AppState>) -> Result<ChatSession, String> {
    let sessions = state.sessions.lock().unwrap();
    let current_id = state.current_session_id.lock().unwrap();
    if let Some(id) = &*current_id {
        if let Some(session) = sessions.get(id) {
            return Ok(session.clone());
        }
    }
    Err("No current session".to_string())
}

#[tauri::command]
async fn finalize_and_new_chat(state: State<'_, AppState>) -> Result<String, String> {
    let mut sessions = state.sessions.lock().unwrap();
    let mut current_id_guard = state.current_session_id.lock().unwrap();

    // 先判断并处理旧的会话
    if let Some(old_id) = current_id_guard.clone() {
        let is_empty = sessions.get(&old_id).map_or(true, |s| s.messages.is_empty());

        if is_empty {
            // 如果旧会话是空的，就从内存中移除
            sessions.remove(&old_id);
        } else {
            // 如果旧会话非空，就更新并保存它
            if let Some(session) = sessions.get_mut(&old_id) {
                if session.title == "New Chat" {
                    session.title = generate_session_title(&session.messages);
                }
                session.updated_at = now_ts();
                save_session_to_file(session)?;
            }
        }
    }

    // 总是创建并设置一个新的会话
    let new_id = Uuid::new_v4().to_string();
    let new_session = ChatSession::new(new_id.clone(), "New Chat".to_string());
    sessions.insert(new_id.clone(), new_session);
    *current_id_guard = Some(new_id.clone());

    Ok(new_id)
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
    let current_id = state.current_session_id.lock().unwrap();
    if let Some(id) = &*current_id {
        if let Some(session) = sessions.get(id) {
            Ok(session.messages.clone())
        } else {
            Ok(Vec::new())
        }
    } else {
        Ok(Vec::new())
    }
}

#[tauri::command]
async fn select_session(id_to_select: String, state: State<'_, AppState>) -> Result<ChatSession, String> {
    let mut sessions = state.sessions.lock().unwrap();
    let mut current_id_guard = state.current_session_id.lock().unwrap();

    // 1. 最终确定或移除旧的会话
    if let Some(old_id) = current_id_guard.clone() {
        if old_id != id_to_select {
            let is_empty = sessions.get(&old_id).map_or(true, |s| s.messages.is_empty());

            if is_empty {
                // 如果旧会话是空的，就从内存中移除，不保存
                sessions.remove(&old_id);
            } else {
                // 如果旧会话非空，就更新并保存它
                if let Some(old_session) = sessions.get_mut(&old_id) {
                    if old_session.title == "New Chat" {
                        old_session.title = generate_session_title(&old_session.messages);
                    }
                    old_session.updated_at = now_ts();
                    save_session_to_file(old_session)?;
                }
            }
        }
    }

    // 2. 切换到新会话
    if let Some(new_session) = sessions.get(&id_to_select) {
        *current_id_guard = Some(id_to_select);
        Ok(new_session.clone())
    } else {
        Err("Session to select not found".to_string())
    }
}

fn main() {
    let context = tauri::generate_context!();
    tauri::Builder::default()
        .plugin(tauri_plugin_store::Builder::default().build())
        .setup(|app| {
            let store = StoreBuilder::new(app.handle(), ".settings.dat".parse().unwrap()).build();
            let sessions = load_sessions_from_files();
            app.manage(AppState { 
                store: Mutex::new(store),
                sessions: Mutex::new(sessions),
                current_session_id: Mutex::new(None),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            send_message_to_openai,
            new_chat_session,
            get_all_sessions,
            get_current_session,
            finalize_and_new_chat,
            save_api_key,
            load_api_key,
            clear_chat_history,
            get_chat_context,
            select_session
        ])
        .run(context)
        .expect("error while running tauri application");
}