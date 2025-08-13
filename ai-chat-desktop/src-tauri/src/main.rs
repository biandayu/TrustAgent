#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use async_trait::async_trait;
use rust_mcp_sdk::{McpClient, StdioTransport, TransportOptions};
use rust_mcp_sdk::mcp_client::{client_runtime, ClientHandler, ClientRuntime};
use rust_mcp_sdk::schema::{InitializeRequestParams, Implementation, ClientCapabilities, LATEST_PROTOCOL_VERSION, Tool};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use tauri::State;
use uuid::Uuid;

// --- Configuration Structures ---

#[derive(Debug, Serialize, Deserialize, Clone, Default)]
struct AppConfig {
    openai: OpenAIParams,
    #[serde(rename = "mcpServers")]
    mcp_servers: HashMap<String, McpServerProcessConfig>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct McpServerProcessConfig {
    command: String,
    args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
struct OpenAIParams {
    api_key: String,
    base_url: String,
    model: String,
}

impl Default for OpenAIParams {
    fn default() -> Self {
        Self {
            api_key: "".to_string(),
            base_url: "https://api.openai.com/v1".to_string(),
            model: "gpt-4-turbo".to_string(),
        }
    }
}

// --- Chat Structures ---
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

// --- Application State ---

#[derive(Debug, Serialize, Clone)]
struct McpServerInfo {
    name: String,
    status: String, // "running" or "stopped"
}

struct AppState {
    config: Mutex<AppConfig>,
    sessions: Mutex<HashMap<String, ChatSession>>,
    current_session_id: Mutex<Option<String>>,
    tool_states: Mutex<HashMap<String, bool>>, // Key: "{server_name}/{tool_name}"
    mcp_clients: Mutex<HashMap<String, Arc<ClientRuntime>>>,
    mcp_tools: Mutex<HashMap<String, Vec<Tool>>>,
}

// --- Filesystem and Config Logic ---
fn get_app_data_dir() -> PathBuf {
    let data_dir = dirs_next::data_dir().expect("Failed to find data directory");
    let app_data_dir = data_dir.join("TrustAgent").join("data");
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
    }
    app_data_dir
}

fn get_app_config_path() -> PathBuf {
    let config_dir = dirs_next::config_dir().expect("Failed to find config directory");
    let app_config_dir = config_dir.join("TrustAgent").join("configuration");
    if !app_config_dir.exists() {
        fs::create_dir_all(&app_config_dir).expect("Failed to create app config directory");
    }
    app_config_dir.join("settings.json")
}

fn save_session_to_file(session: &ChatSession) -> Result<(), String> {
    let dir = get_app_data_dir().join(".chats");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
    let path = dir.join(format!("{}.json", session.id));
    let content = serde_json::to_string_pretty(session).map_err(|e| e.to_string())?;
    fs::write(path, content).map_err(|e| e.to_string())?;
    Ok(())
}

fn delete_session_file(session_id: &str) -> Result<(), String> {
    let dir = get_app_data_dir().join(".chats");
    let path = dir.join(format!("{}.json", session_id));
    if path.exists() {
        fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

fn load_sessions_from_files() -> HashMap<String, ChatSession> {
    let dir = get_app_data_dir().join(".chats");
    if !dir.exists() {
        fs::create_dir_all(&dir).ok();
    }
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

fn load_or_initialize_config() -> AppConfig {
    let config_path = get_app_config_path();
    if config_path.exists() {
        let content = fs::read_to_string(&config_path).unwrap_or_default();
        serde_json::from_str(&content).unwrap_or_else(|_| {
            let default_config = AppConfig::default();
            fs::write(
                &config_path,
                serde_json::to_string_pretty(&default_config).unwrap(),
            )
            .ok();
            default_config
        })
    } else {
        let default_config = AppConfig::default();
        fs::write(
            &config_path,
            serde_json::to_string_pretty(&default_config).unwrap(),
        )
        .expect("Failed to write default config file");
        default_config
    }
}

fn generate_session_title(messages: &[ChatMessage]) -> String {
    messages
        .iter()
        .find(|m| m.role == "user")
        .map(|m| {
            let mut t = m.content.trim().to_string();
            if t.len() > 20 {
                t.truncate(20);
                t.push_str("...");
            }
            t
        })
        .unwrap_or_else(|| "New Chat".to_string())
}

// --- Tauri MCP Commands ---

#[tauri::command]
fn get_mcp_servers(state: State<'_, AppState>) -> Result<Vec<McpServerInfo>, String> {
    let config = state.config.lock().unwrap();
    let clients = state.mcp_clients.lock().unwrap();
    let servers_info = config
        .mcp_servers
        .keys()
        .map(|name| McpServerInfo {
            name: name.clone(),
            status: if clients.contains_key(name) {
                "running".to_string()
            } else {
                "stopped".to_string()
            },
        })
        .collect();
    Ok(servers_info)
}

#[tauri::command]
async fn start_mcp_server(server_name: String, state: State<'_, AppState>) -> Result<(), String> {
    let config = state.config.lock().unwrap().clone();
    let server_config = config
        .mcp_servers
        .get(&server_name)
        .ok_or_else(|| "Server config not found".to_string())?;

    // Step 1: 构造 client details
    let client_details = InitializeRequestParams {
        capabilities: ClientCapabilities::default(),
        client_info: Implementation {
            name: format!("tauri-mcp-client-{}", server_name),
            version: "0.1.0".into(),
            title: Some(format!("tauri-mcp-client-{}", server_name)),
        },
        protocol_version: LATEST_PROTOCOL_VERSION.into(),
    };

    // Step 2: 启动 MCP Server 子进程并创建 stdio transport
    let transport = StdioTransport::create_with_server_launch(
        &server_config.command,
        server_config.args.clone(),
        None,
        TransportOptions::default(),
    ).map_err(|e| e.to_string())?;

    // Step 3: 实现 handler
    struct MyClientHandler;
    #[async_trait]
    impl ClientHandler for MyClientHandler {}
    let handler = MyClientHandler {};

    // Step 4: 创建 MCP client
    let client = client_runtime::create_client(client_details, transport, handler);

    // Step 5: 启动 client
    McpClient::start(client.clone()).await.map_err(|e| e.to_string())?;

    // Step 6: 获取工具列表
    let tools = McpClient::list_tools(&*client, None).await
        .map_err(|e| format!("Failed to get tools: {e}"))?
        .tools;

    state
        .mcp_clients
        .lock()
        .unwrap()
        .insert(server_name.clone(), client.clone());
    state
        .mcp_tools
        .lock()
        .unwrap()
        .insert(server_name.clone(), tools);

    Ok(())
}

#[tauri::command]
async fn stop_mcp_server(server_name: String, state: State<'_, AppState>) -> Result<(), String> {
    // 先移除 client，释放锁
    let client = {
        let mut clients = state.mcp_clients.lock().unwrap();
        clients.remove(&server_name)
    };
    if let Some(client) = client {
        McpClient::shut_down(&*client).await.map_err(|e| e.to_string())?;
        state.mcp_tools.lock().unwrap().remove(&server_name);
    }
    Ok(())
}

#[tauri::command]
fn get_discovered_tools(server_name: String, state: State<'_, AppState>) -> Result<Vec<Tool>, String> {
    let tools = state.mcp_tools.lock().unwrap();
    Ok(tools.get(&server_name).cloned().unwrap_or_default())
}

// --- Tauri Session Commands ---

#[tauri::command]
fn rename_session(id: String, new_title: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if let Some(session) = sessions.get_mut(&id) {
        session.title = new_title;
        session.updated_at = now_ts();
        save_session_to_file(session)?;
    }
    Ok(())
}

#[tauri::command]
fn delete_session(id: String, state: State<'_, AppState>) -> Result<(), String> {
    let mut sessions = state.sessions.lock().unwrap();
    if sessions.remove(&id).is_some() {
        delete_session_file(&id)?;
    }
    Ok(())
}

#[tauri::command]
fn open_config_file() -> Result<(), String> {
    let path = get_app_config_path();
    opener::open(&path).map_err(|e| format!("Failed to open config file: {}", e))
}

#[tauri::command]
async fn send_message_to_openai(
    message: String,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let config = state.config.lock().unwrap().clone();

    if config.openai.api_key.is_empty() {
        return Err("OpenAI API key is not set in the configuration file.".to_string());
    }

    let openai_config = OpenAIConfig::new()
        .with_api_key(config.openai.api_key)
        .with_api_base(config.openai.base_url);

    let client = Client::with_config(openai_config);

    let openai_msgs = {
        let mut sessions = state.sessions.lock().unwrap();
        let mut current_id = state.current_session_id.lock().unwrap();
        let session_id = current_id.clone().unwrap_or_else(|| {
            let id = Uuid::new_v4().to_string();
            sessions.insert(
                id.clone(),
                ChatSession::new(id.clone(), "New Chat".to_string()),
            );
            *current_id = Some(id.clone());
            id
        });
        let session = sessions.get_mut(&session_id).unwrap();

        session.messages.push(ChatMessage {
            role: "user".to_string(),
            content: message.clone(),
            timestamp: now_ts(),
        });
        session.updated_at = now_ts();

        if session.messages.iter().all(|m| m.role != "system") {
            session.messages.insert(
                0,
                ChatMessage {
                    role: "system".to_string(),
                    content: "You are a helpful AI assistant.".to_string(),
                    timestamp: now_ts(),
                },
            );
        }

        session
            .messages
            .iter()
            .map(|msg| match msg.role.as_str() {
                "system" => ChatCompletionRequestSystemMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
                "user" => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
                "assistant" => ChatCompletionRequestAssistantMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
                _ => ChatCompletionRequestUserMessageArgs::default()
                    .content(msg.content.clone())
                    .build()
                    .unwrap()
                    .into(),
            })
            .collect::<Vec<_>>()
    };

    let request = CreateChatCompletionRequestArgs::default()
        .model(config.openai.model)
        .messages(openai_msgs)
        .build()
        .map_err(|e| e.to_string())?;

    let response = client
        .chat()
        .create(request)
        .await
        .map_err(|e| e.to_string())?;
    let assistant_message = response
        .choices
        .get(0)
        .and_then(|choice| choice.message.content.clone())
        .unwrap_or_else(|| "No response received".to_string());

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
async fn get_all_sessions(state: State<'_, AppState>) -> Result<Vec<ChatSession>, String> {
    let sessions = state.sessions.lock().unwrap();
    let mut list: Vec<_> = sessions.values().cloned().collect();
    list.sort_by_key(|s| std::cmp::Reverse(s.updated_at));
    Ok(list)
}

#[tauri::command]
async fn finalize_and_new_chat(state: State<'_, AppState>) -> Result<String, String> {
    let mut sessions = state.sessions.lock().unwrap();
    let mut current_id_guard = state.current_session_id.lock().unwrap();

    if let Some(old_id) = current_id_guard.clone() {
        let is_empty = sessions
            .get(&old_id)
            .map_or(true, |s| s.messages.is_empty());

        if is_empty {
            sessions.remove(&old_id);
        } else {
            if let Some(session) = sessions.get_mut(&old_id) {
                if session.title == "New Chat" {
                    session.title = generate_session_title(&session.messages);
                }
                session.updated_at = now_ts();
                save_session_to_file(session)?;
            }
        }
    }

    let new_id = Uuid::new_v4().to_string();
    let new_session = ChatSession::new(new_id.clone(), "New Chat".to_string());
    sessions.insert(new_id.clone(), new_session);
    *current_id_guard = Some(new_id.clone());

    Ok(new_id)
}

#[tauri::command]
async fn select_session(
    id_to_select: String,
    state: State<'_, AppState>,
) -> Result<ChatSession, String> {
    let mut sessions = state.sessions.lock().unwrap();
    let mut current_id_guard = state.current_session_id.lock().unwrap();

    if let Some(old_id) = current_id_guard.clone() {
        if old_id != id_to_select {
            let is_empty = sessions
                .get(&old_id)
                .map_or(true, |s| s.messages.is_empty());

            if is_empty {
                sessions.remove(&old_id);
            } else {
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

    if let Some(new_session) = sessions.get(&id_to_select) {
        *current_id_guard = Some(id_to_select);
        Ok(new_session.clone())
    } else {
        Err("Session to select not found".to_string())
    }
}

fn main() {
    let config = load_or_initialize_config();
    let sessions = load_sessions_from_files();

    tauri::Builder::default()
        .manage(AppState {
            config: Mutex::new(config),
            sessions: Mutex::new(sessions),
            current_session_id: Mutex::new(None),
            tool_states: Mutex::new(HashMap::new()),
            mcp_clients: Mutex::new(HashMap::new()),
            mcp_tools: Mutex::new(HashMap::new()),
        })
        .invoke_handler(tauri::generate_handler![
            // MCP
            get_mcp_servers,
            start_mcp_server,
            stop_mcp_server,
            get_discovered_tools,
            // Session
            send_message_to_openai,
            get_all_sessions,
            finalize_and_new_chat,
            select_session,
            open_config_file,
            rename_session,
            delete_session
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}