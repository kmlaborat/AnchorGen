use reqwest;
use serde::{Deserialize, Serialize};

#[derive(Debug)]
pub struct LlmConfig {
    pub base_url: String,
    pub api_key: String,
}

pub fn resolve_llm_config() -> Result<LlmConfig, String> {
    let base_url = std::env::var("ANCHORGEN_BASE_URL")
        .map_err(|_| "Missing ANCHORGEN_BASE_URL")?;
    let api_key = std::env::var("ANCHORGEN_API_KEY")
        .map_err(|_| "Missing ANCHORGEN_API_KEY")?;

    Ok(LlmConfig { base_url, api_key })
}

#[derive(Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<ChatMessage>,
}

#[derive(Serialize)]
struct ChatMessage {
    role: String,
    content: String,
}

#[derive(Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Deserialize)]
struct Message {
    content: String,
}

pub fn generate(prompt: &str, config: &LlmConfig, model: &str) -> Result<String, String> {
    let client = reqwest::blocking::Client::new();

    let url = format!("{}/v1/chat/completions", config.base_url);

    let request = ChatRequest {
        model: model.to_string(),
        messages: vec![ChatMessage {
            role: "user".to_string(),
            content: prompt.to_string(),
        }],
    };

    let response = client
        .post(&url)
        .header("Authorization", &format!("Bearer {}", config.api_key))
        .json(&request)
        .send()
        .map_err(|e| format!("LLM request failed: {}", e))?;

    if !response.status().is_success() {
        return Err(format!("LLM API returned status: {}", response.status()));
    }

    let chat_response: ChatResponse = response
        .json()
        .map_err(|e| format!("Failed to parse LLM response: {}", e))?;

    chat_response
        .choices
        .first()
        .map(|c| c.message.content.trim().to_string())
        .ok_or("LLM response contains no choices".to_string())
}
