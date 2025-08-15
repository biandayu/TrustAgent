//! The core Agent logic module.

use crate::AppState;
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
use tracing::{info, instrument};

// Represents an MCP tool that the Agent can use.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tool {
    pub server_name: String,
    pub tool_name: String,
    // A description for the LLM to understand what the tool does.
    pub description: String,
}

#[derive(Debug, Serialize, Deserialize)]
struct ToolCall {
    tool_name: String,
    arguments: serde_json::Value,
}

// The main struct for executing a user's request.
pub struct Agent {}

impl Agent {
    pub fn new() -> Self {
        Self {}
    }

    // The main execution loop for the agent.
    #[instrument(skip(self, prompt, available_tools, state))]
    pub async fn run_task(
        &self,
        prompt: String,
        available_tools: Vec<Tool>,
        state: Arc<AppState>,
    ) -> Result<String, String> {
        info!(prompt = %prompt, num_tools = available_tools.len(), "Running agent task");

        // Clone the necessary data before the loop to avoid holding locks across await points.
        let config = state.config.lock().unwrap().clone();
        let mcp_clients_clone = state.mcp_clients.lock().unwrap().clone();
        
        if config.openai.api_key.is_empty() {
            return Err("OpenAI API key is not set in the configuration file.".to_string());
        }

        let openai_config = OpenAIConfig::new()
            .with_api_key(config.openai.api_key)
            .with_api_base(config.openai.base_url);
        let openai_client = Client::with_config(openai_config);

        let tool_list_str = available_tools
            .iter()
            .map(|t| format!("- {}: {}", t.tool_name, t.description))
            .collect::<Vec<_>>()
            .join("\n");

        let system_prompt = format!(
            "You are a helpful AI assistant that can use tools to answer questions.\n\nAvailable tools:\n{}\n\nWhen you need to use a tool, respond ONLY with a JSON object in the following format: {{ \"tool_name\": \"<tool_name>\", \"arguments\": {{<arguments>}} }}. Do not include any other text or explanations. When you have the final answer, respond with the answer directly as plain text.",
            tool_list_str
        );

        let mut messages: Vec<ChatCompletionRequestMessage> = vec![
            ChatCompletionRequestSystemMessageArgs::default()
                .content(system_prompt)
                .build()
                .unwrap()
                .into(),
            ChatCompletionRequestUserMessageArgs::default()
                .content(prompt)
                .build()
                .unwrap()
                .into(),
        ];

        const MAX_ITERATIONS: u32 = 5;

        for i in 0..MAX_ITERATIONS {
            info!(iteration = i + 1, "Agent loop iteration");

            let request = CreateChatCompletionRequestArgs::default()
                .model(config.openai.model.clone())
                .messages(messages.clone())
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

            // Try to parse the response as a tool call
            if let Ok(tool_call) = serde_json::from_str::<ToolCall>(&assistant_message) {
                info!(tool_name = %tool_call.tool_name, "LLM requested a tool call");

                // Find the server for the requested tool
                let tool_info = available_tools
                    .iter()
                    .find(|t| t.tool_name == tool_call.tool_name)
                    .ok_or_else(|| format!("Tool '{}' not found.", tool_call.tool_name))?;

                // Get the MCP client for that server from our cloned map
                let mcp_client = mcp_clients_clone
                    .get(&tool_info.server_name)
                    .ok_or_else(|| format!("MCP client for server '{}' not found or not running.", tool_info.server_name))?;

                // Execute the tool
                // The mcp_client is an Arc<RunningService<RoleClient, ()>>.
                // RunningService implements Deref<Target = Peer<RoleClient>>,
                // so we can call Peer methods directly on it or via .as_ref().
                info!(tool_name = %tool_call.tool_name, args = ?tool_call.arguments, "Executing tool");
                
                // Convert serde_json::Value arguments to JsonObject (Map<String, Value>)
                let arguments_object: Option<JsonObject> = match tool_call.arguments {
                    serde_json::Value::Object(map) => Some(map),
                    serde_json::Value::Null => None,
                    _ => {
                        // If arguments is not an object or null, log a warning and pass null.
                        // This might happen if the LLM provides invalid JSON or a non-object type.
                        tracing::warn!("Tool arguments for '{}' are not a JSON object or null. Arguments: {:?}", tool_call.tool_name, tool_call.arguments);
                        None
                    }
                };

                // Create the parameter struct for the call_tool request
                // We need to convert the tool name to a Cow<'static, str>.
                // Since `tool_call.tool_name` is a String owned by `tool_call`, we can move it.
                let tool_name_cow: Cow<'static, str> = Cow::Owned(tool_call.tool_name.clone());
                
                let param = CallToolRequestParam {
                    name: tool_name_cow,
                    arguments: arguments_object,
                    // Note: There might be other fields in CallToolRequestParam depending on the rmcp version.
                    // If compilation fails due to missing fields, check the struct definition in the generated docs.
                };

                // Call the tool on the MCP server
                let tool_result = mcp_client
                    .as_ref() // Get &Peer<RoleClient> from Arc<Peer<RoleClient>>
                    .call_tool(param) // Pass the CallToolRequestParam struct
                    .await;

                let result_str = match tool_result {
                    Ok(call_result) => {
                        // `call_result` is of type CallToolResult.
                        // It contains `content: Option<Vec<Content>>` and `is_error: bool`.
                        // For simplicity, we'll serialize the entire result.
                        // A more advanced agent could process the `Content` types (text, image, resource, etc.) individually.
                        serde_json::to_string(&call_result).unwrap_or_else(|e| format!("Failed to serialize tool result: {}", e))
                    }
                    Err(service_error) => {
                        // `service_error` is rmcp::service::ServiceError.
                        // It can wrap various kinds of errors, including McpError for protocol-level issues.
                        format!("Tool execution failed: {:?}", service_error)
                    }
                };
                info!(tool_name = %tool_call.tool_name, result = %result_str, "Tool execution finished");

                // Add the tool call and result to the conversation history
                messages.push(
                    ChatCompletionRequestAssistantMessageArgs::default()
                        .content(assistant_message)
                        .build()
                        .unwrap()
                        .into(),
                );
                messages.push(
                    ChatCompletionRequestUserMessageArgs::default() // Using User role for tool result for simplicity
                        .content(format!("Tool result for '{}':\n{}", tool_call.tool_name, result_str))
                        .build()
                        .unwrap()
                        .into(),
                );
                // Continue the loop to let the LLM process the tool's result
                continue;
            } else {
                // If it's not a tool call, it's the final answer
                info!("LLM provided a final answer.");
                return Ok(assistant_message);
            }
        }

        Err("Agent exceeded maximum iterations.".to_string())
    }
}
