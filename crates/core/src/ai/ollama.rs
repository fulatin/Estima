use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::provider::{AIProvider, ChatRequest, ChatResponse};

#[derive(Debug, Serialize)]
struct OllamaRequest {
    model: String,
    messages: Vec<OllamaMessage>,
    stream: bool,
}

#[derive(Debug, Serialize, Deserialize)]
struct OllamaMessage {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct OllamaResponse {
    message: OllamaMessage,
}

pub struct OllamaProvider {
    client: Client,
    base_url: String,
    model: String,
}

impl OllamaProvider {
    pub fn new(base_url: Option<&str>, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.unwrap_or("http://localhost:11434").to_string(),
            model: model.unwrap_or("llama3").to_string(),
        }
    }

    pub async fn is_available(&self) -> bool {
        let url = format!("{}/api/tags", self.base_url);
        self.client.get(&url).send().await.is_ok()
    }

    pub async fn list_models(&self) -> Result<Vec<String>> {
        let url = format!("{}/api/tags", self.base_url);
        let response = self.client.get(&url).send().await?;
        let json: serde_json::Value = response.json().await?;
        
        let models = json["models"]
            .as_array()
            .map(|arr| {
                arr.iter()
                    .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
                    .collect()
            })
            .unwrap_or_default();
        
        Ok(models)
    }
}

#[async_trait]
impl AIProvider for OllamaProvider {
    async fn chat(&self, message: &str, system_prompt: Option<&str>) -> Result<String> {
        log::debug!("[Ollama] Starting chat request");
        log::debug!("[Ollama] Model: {}", self.model);
        log::debug!("[Ollama] Message: {}", message);
        
        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(OllamaMessage {
                role: "system".to_string(),
                content: prompt.to_string(),
            });
        }
        
        messages.push(OllamaMessage {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let request = OllamaRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        log::debug!("[Ollama] Sending request to {}", self.base_url);

        let url = format!("{}/api/chat", self.base_url);
        let response = self.client
            .post(&url)
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Ollama] Connection failed: {}", e);
                anyhow!("Failed to connect to Ollama: {}", e)
            })?;

        log::debug!("[Ollama] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("[Ollama] API error: {} - {}", status, body);
            return Err(anyhow!("Ollama API error: {} - {}", status, body));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Ollama response: {}", e))?;

        log::debug!("[Ollama] Response: {}", ollama_response.message.content.chars().take(200).collect::<String>());

        Ok(ollama_response.message.content)
    }

    fn provider_name(&self) -> &str {
        "Ollama"
    }

    async fn chat_with_tools(&self, request: ChatRequest) -> Result<ChatResponse> {
        log::debug!("[Ollama] Starting chat_with_tools request");
        log::debug!("[Ollama] Model: {}", self.model);
        log::debug!("[Ollama] Messages count: {}", request.messages.len());
        
        let mut messages = Vec::new();
        
        if let Some(ref prompt) = request.system_prompt {
            messages.push(OllamaMessage {
                role: "system".to_string(),
                content: prompt.to_string(),
            });
        }
        
        for msg in &request.messages {
            messages.push(OllamaMessage {
                role: match msg.role {
                    super::provider::MessageRole::System => "system",
                    super::provider::MessageRole::User => "user",
                    super::provider::MessageRole::Assistant => "assistant",
                    super::provider::MessageRole::Tool => "user",
                }.to_string(),
                content: if msg.role == super::provider::MessageRole::Tool {
                    format!("[Tool result]: {}", msg.content)
                } else {
                    msg.content.clone()
                },
            });
        }

        let ollama_request = OllamaRequest {
            model: self.model.clone(),
            messages,
            stream: false,
        };

        log::debug!("[Ollama] Sending request to {}", self.base_url);

        let url = format!("{}/api/chat", self.base_url);
        let response = self.client
            .post(&url)
            .json(&ollama_request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[Ollama] Connection failed: {}", e);
                anyhow!("Failed to connect to Ollama: {}", e)
            })?;

        log::debug!("[Ollama] Response status: {}", response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("[Ollama] API error: {} - {}", status, body);
            return Err(anyhow!("Ollama API error: {} - {}", status, body));
        }

        let ollama_response: OllamaResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse Ollama response: {}", e))?;

        log::debug!("[Ollama] Response: {}", ollama_response.message.content.chars().take(200).collect::<String>());

        Ok(ChatResponse {
            content: Some(ollama_response.message.content),
            tool_calls: None,
            reasoning_content: None,
        })
    }
}
