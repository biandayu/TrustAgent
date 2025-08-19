use crate::ChatMessage;

const MAX_MESSAGES: usize = 40; // Adjust this based on your needs

pub fn select_context_messages(messages: &[ChatMessage], max_messages: Option<usize>) -> Vec<ChatMessage> {
    let window_size = max_messages.unwrap_or(MAX_MESSAGES);
    
    let mut result = Vec::new();
    
    // Always include system messages as they set up important context
    let system_messages: Vec<_> = messages.iter()
        .filter(|m| m.role == "system")
        .cloned()
        .collect();
    
    result.extend(system_messages);
    
    // Get the most recent N messages that aren't system messages
    let recent_messages: Vec<_> = messages.iter()
        .rev() // Reverse to get most recent first
        .filter(|m| m.role != "system")
        .take(window_size)
        .cloned()
        .collect();
    
    // Add them in chronological order
    result.extend(recent_messages.into_iter().rev());
    
    result
}

// Optional: Implement the summarization mechanism
pub fn summarize_old_messages(_messages: &[ChatMessage]) -> Option<String> {
    // TODO: Implement message summarization using LLM
    // This would create a summary of older messages to preserve context
    // while keeping the token count low
    None
}

// Async version that will be used when we implement LLM summarization
pub async fn summarize_old_messages_async(_messages: &[ChatMessage]) -> Option<String> {
    summarize_old_messages(_messages)
}
