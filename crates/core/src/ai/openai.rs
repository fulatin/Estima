use async_trait::async_trait;
use anyhow::{anyhow, Result};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use super::AIProvider;

#[derive(Debug, Serialize)]
struct ChatRequest {
    model: String,
    messages: Vec<Message>,
}

#[derive(Debug, Serialize, Deserialize)]
struct Message {
    role: String,
    content: String,
}

#[derive(Debug, Deserialize)]
struct ChatResponse {
    choices: Vec<Choice>,
}

#[derive(Debug, Deserialize)]
struct Choice {
    message: Message,
}

#[derive(Debug, Clone)]
pub enum AIProviderType {
    OpenAI,
    SiliconFlow,
    DeepSeek,
    Anthropic,
    Custom(String),
}

impl AIProviderType {
    pub fn base_url(&self) -> String {
        match self {
            AIProviderType::OpenAI => "https://api.openai.com/v1".to_string(),
            AIProviderType::SiliconFlow => "https://api.siliconflow.cn/v1".to_string(),
            AIProviderType::DeepSeek => "https://api.deepseek.com/v1".to_string(),
            AIProviderType::Anthropic => "https://api.anthropic.com/v1".to_string(),
            AIProviderType::Custom(url) => url.clone(),
        }
    }

    pub fn default_model(&self) -> &'static str {
        match self {
            AIProviderType::OpenAI => "gpt-4o-mini",
            AIProviderType::SiliconFlow => "Qwen/Qwen2.5-7B-Instruct",
            AIProviderType::DeepSeek => "deepseek-chat",
            AIProviderType::Anthropic => "claude-3-haiku-20240307",
            AIProviderType::Custom(_) => "gpt-3.5-turbo",
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            AIProviderType::OpenAI => "OpenAI",
            AIProviderType::SiliconFlow => "SiliconFlow",
            AIProviderType::DeepSeek => "DeepSeek",
            AIProviderType::Anthropic => "Anthropic",
            AIProviderType::Custom(name) => name,
        }
    }
}

pub struct OpenAICompatibleProvider {
    client: Client,
    api_key: String,
    model: String,
    base_url: String,
    provider_type: AIProviderType,
}

impl OpenAICompatibleProvider {
    pub fn new(provider_type: AIProviderType, api_key: &str, model: Option<&str>) -> Self {
        Self {
            client: Client::new(),
            api_key: api_key.to_string(),
            model: model.unwrap_or(provider_type.default_model()).to_string(),
            base_url: provider_type.base_url(),
            provider_type,
        }
    }

    pub fn from_env() -> Result<Self> {
        let (provider_type, api_key) = Self::detect_provider_from_env()?;
        let model = std::env::var("AI_MODEL")
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .ok();
        
        Ok(Self::new(provider_type, &api_key, model.as_deref()))
    }

    fn detect_provider_from_env() -> Result<(AIProviderType, String)> {
        if let Ok(api_key) = std::env::var("SILICONFLOW_API_KEY") {
            return Ok((AIProviderType::SiliconFlow, api_key));
        }
        
        if let Ok(api_key) = std::env::var("DEEPSEEK_API_KEY") {
            return Ok((AIProviderType::DeepSeek, api_key));
        }
        
        if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
            return Ok((AIProviderType::Anthropic, api_key));
        }
        
        if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
            return Ok((AIProviderType::OpenAI, api_key));
        }
        
        if let Ok(api_key) = std::env::var("AI_API_KEY") {
            let base_url = std::env::var("AI_BASE_URL")
                .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
            return Ok((AIProviderType::Custom(base_url), api_key));
        }
        
        Err(anyhow!(
            "No AI API key found. Set one of: SILICONFLOW_API_KEY, DEEPSEEK_API_KEY, ANTHROPIC_API_KEY, OPENAI_API_KEY, or AI_API_KEY"
        ))
    }
}

#[async_trait]
impl AIProvider for OpenAICompatibleProvider {
    async fn chat(&self, message: &str, system_prompt: Option<&str>) -> Result<String> {
        let mut messages = Vec::new();
        
        if let Some(prompt) = system_prompt {
            messages.push(Message {
                role: "system".to_string(),
                content: prompt.to_string(),
            });
        }
        
        messages.push(Message {
            role: "user".to_string(),
            content: message.to_string(),
        });

        let request = ChatRequest {
            model: self.model.clone(),
            messages,
        };

        let url = format!("{}/chat/completions", self.base_url);
        let response = self.client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| anyhow!("Failed to connect to {}: {}", self.provider_type.display_name(), e))?;

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            return Err(anyhow!("{} API error: {} - {}", self.provider_type.display_name(), status, body));
        }

        let chat_response: ChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        chat_response
            .choices
            .first()
            .map(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from {}", self.provider_type.display_name()))
    }

    fn provider_name(&self) -> &str {
        self.provider_type.display_name()
    }
}

pub type OpenAIProvider = OpenAICompatibleProvider;
