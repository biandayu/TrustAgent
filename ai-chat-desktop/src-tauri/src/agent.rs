//! The core Agent logic module.

use crate::{AppState, ChatMessage};
use async_openai::{
    config::OpenAIConfig,
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use rmcp::model::{CallToolRequestParam, JsonObject};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::sync::Arc;
use tauri::Window;
use tracing::{info, instrument, warn};

// --- Agent Event Structures ---

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub status: AgentStatus,
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", content = "data", rename_all = "camelCase")]
pub enum AgentStatus {
    Thinking,
    UsingTool {
        tool_name: String,
    },
}

// --- Agent Core Structures ---

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub server_name: String,
    pub tool_name: String,
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCall {
    tool_name: String,
    arguments: serde_json::Value,
}

pub struct Agent {}

/// Extracts a JSON object from a string that might contain other text or markdown fences.
fn extract_json_from_str(s: &str) -> Option<&str> {
    // Find the first '{' which often marks the beginning of a JSON object.
    let start_byte = s.find('{')?;
    // Find the last '}' which often marks the end of a JSON object.
    let end_byte = s.rfind('}')?;

    if start_byte >= end_byte {
        return None;
    }

    // Return the slice between the first '{' and the last '}' (inclusive).
    Some(&s[start_byte..=end_byte])
}

impl Agent {
    pub fn new() -> Self {
        Self {}
    }

    #[instrument(skip(self, history, available_tools, state, window))]
    pub async fn run_task(
        &self,
        history: &[
            ChatMessage
        ],
        available_tools: Vec<Tool>,
        state: Arc<AppState>,
        window: &Window,
    ) -> Result<String, String> {
        info!(num_messages = history.len(), num_tools = available_tools.len(), "Running agent task");

        let config = state.config.lock().unwrap().clone();
        let mcp_clients_clone = state.mcp_clients.lock().unwrap().clone();
        
        if config.openai.api_key.is_empty() {
            return Err("OpenAI API key is not set in the configuration file.".to_string());
        }

        let openai_config = OpenAIConfig::new()
            .with_api_key(config.openai.api_key)
            .with_api_base(config.openai.base_url);
        let openai_client = Client::with_config(openai_config);

        let system_prompt = if available_tools.is_empty() {
            "You are a helpful AI assistant.".to_string()
        } else {
            let tool_list_str = available_tools
                .iter()
                .map(|t| format!("- {}: {}", t.tool_name, t.description))
                .collect::<Vec<_>>()
                .join("\n");

            format!(
                "You are a powerful AI assistant capable of using tools to answer questions. You have access to the following tools:\n\n{}\n\nTo use a tool, you MUST respond with ONLY a single JSON object with two keys: 'tool_name' and 'arguments'. For example: {{'tool_name': 'read_file', 'arguments': {{'path': '/path/to/file.txt'}}}}\nDo not provide any other text, conversation, or explanation before or after the JSON object. If you have the final answer for the user, provide it directly as plain text without any JSON.",
                tool_list_str
            )
        };

        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .unwrap()
                .into(),
        ];

        for msg in history {
            match msg.role.as_str() {
                "user" => messages.push(
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(&*msg.content)
                        .build()
                        .unwrap()
                        .into(),
                ),
                "assistant" => messages.push(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(&*msg.content)
                        .build()
                        .unwrap()
                        .into(),
                ),
                _ => (),
            }
        }

        const MAX_ITERATIONS: u32 = 20;
        const CONTEXT_WINDOW_SIZE: usize = 20;

        for i in 0..MAX_ITERATIONS {
            info!(iteration = i + 1, "Agent loop iteration");

            window
                .emit(
                    "agent_event",
                    AgentEvent {
                        status: AgentStatus::Thinking,
                    },
                )
                .ok();

            let final_messages = if messages.len() > CONTEXT_WINDOW_SIZE {
                info!(
                    "Message history length ({}) exceeds context window size ({}). Truncating.",
                    messages.len(),
                    CONTEXT_WINDOW_SIZE
                );
                let mut truncated_messages = vec![messages[0].clone()];
                let recent_messages = messages.iter().skip(messages.len() - CONTEXT_WINDOW_SIZE);
                truncated_messages.extend(recent_messages.cloned());
                truncated_messages
            } else {
                messages.clone()
            };

            let request = CreateChatCompletionRequestArgs::default()
                .model(config.openai.model.clone())
                .messages(final_messages)
                .build()
                .map_err(|e| e.to_string())?;

            let response = openai_client
                .chat()
                .create(request)
                .await
                .map_err(|e| e.to_string())?;

            let assistant_message = response
                .choices
                .get(0)
                .and_then(|choice| choice.message.content.clone())
                .unwrap_or_else(|| "No response received".to_string());

            if let Some(json_str) = extract_json_from_str(&assistant_message) {
                if let Ok(tool_call) = serde_json::from_str::<ToolCall>(json_str) {
                    info!(tool_name = %tool_call.tool_name, "LLM requested a tool call");

                    window
                        .emit(
                            "agent_event",
                            AgentEvent {
                                status: AgentStatus::UsingTool {
                                    tool_name: tool_call.tool_name.clone(),
                                },
                            },
                        )
                        .ok();

                    let tool_info = available_tools
                        .iter()
                        .find(|t| t.tool_name == tool_call.tool_name)
                        .ok_or_else(|| format!("Tool '{}' not found.", tool_call.tool_name))?;

                    let mcp_client = mcp_clients_clone
                        .get(&tool_info.server_name)
                        .ok_or_else(|| format!("MCP client for server '{}' not found or not running.", tool_info.server_name))?;

                    info!(tool_name = %tool_call.tool_name, args = ?tool_call.arguments, "Executing tool");
                    
                    let arguments_object: Option<JsonObject> = match tool_call.arguments {
                        serde_json::Value::Object(map) => Some(map),
                        serde_json::Value::Null => None,
                        _ => {
                            warn!("Tool arguments for '{}' are not a JSON object or null. Arguments: {}", tool_call.tool_name, tool_call.arguments);
                            None
                        }
                    };

                    let tool_name_cow: Cow<'static, str> = Cow::Owned(tool_call.tool_name.clone());
                    
                    let param = CallToolRequestParam {
                        name: tool_name_cow,
                        arguments: arguments_object,
                    };

                    let tool_result = mcp_client
                        .as_ref()
                        .call_tool(param)
                        .await;

                    let result_str = match tool_result {
                        Ok(call_result) => {
                            serde_json::to_string(&call_result).unwrap_or_else(|e| format!("Failed to serialize tool result: {}", e))
                        }
                        Err(service_error) => {
                            format!("Tool execution failed: {:?}", service_error)
                        }
                    };
                    info!(tool_name = %tool_call.tool_name, result = %result_str, "Tool execution finished");

                    messages.push(
                        ChatCompletionRequestAssistantMessageArgs::default()
                            .content(assistant_message)
                            .build()
                            .unwrap()
                            .into(),
                    );
                    messages.push(
                        ChatCompletionRequestUserMessageArgs::default()
                            .content(format!("Tool result for '{}':\n{}", tool_call.tool_name, result_str))
                            .build()
                            .unwrap()
                            .into(),
                    );
                    continue;
                }
            }
            
            info!("LLM provided a final answer.");
            return Ok(assistant_message);
        }

        Err("Agent exceeded maximum iterations.".to_string())
    }
}
