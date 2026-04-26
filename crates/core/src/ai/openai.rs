use super::provider::{AIProvider, ChatMessage, ChatRequest, ChatResponse, MessageRole};
use super::tools::{FunctionCallRequest, ToolCallRequest};
use anyhow::{Result, anyhow};
use async_trait::async_trait;
use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize)]
struct OpenAIChatRequest {
    model: String,
    messages: Vec<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    tools: Option<Vec<serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChatResponse {
    choices: Vec<OpenAIChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenAIChoice {
    message: OpenAIMessage,
    finish_reason: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIMessage {
    role: String,
    content: Option<String>,
    #[serde(default)]
    tool_calls: Vec<OpenAIToolCall>,
    #[serde(default)]
    reasoning_content: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OpenAIToolCall {
    id: String,
    #[serde(rename = "type")]
    call_type: String,
    function: OpenAIFunctionCall,
}

#[derive(Debug, Deserialize)]
struct OpenAIFunctionCall {
    name: String,
    arguments: String,
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
            AIProviderType::SiliconFlow => "Qwen/Qwen2.5-72B-Instruct",
            AIProviderType::DeepSeek => "deepseek-v4-pro",
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

    pub fn supports_tools(&self) -> bool {
        matches!(
            self,
            AIProviderType::OpenAI | AIProviderType::SiliconFlow | AIProviderType::DeepSeek
        )
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

    fn message_to_json(&self, msg: &ChatMessage) -> serde_json::Value {
        let mut map = serde_json::Map::new();
        map.insert(
            "role".to_string(),
            serde_json::json!(match msg.role {
                MessageRole::System => "system",
                MessageRole::User => "user",
                MessageRole::Assistant => "assistant",
                MessageRole::Tool => "tool",
            }),
        );

        if msg.role == MessageRole::Assistant || !msg.content.is_empty() {
            map.insert("content".to_string(), serde_json::json!(&msg.content));
        }

        if let Some(ref name) = msg.name {
            map.insert("name".to_string(), serde_json::json!(name));
        }

        if let Some(ref id) = msg.tool_call_id {
            map.insert("tool_call_id".to_string(), serde_json::json!(id));
        }

        if let Some(ref calls) = msg.tool_calls {
            map.insert("tool_calls".to_string(), serde_json::json!(calls));
        }

        if let Some(ref reasoning) = msg.reasoning_content {
            map.insert("reasoning_content".to_string(), serde_json::json!(reasoning));
        }

        serde_json::Value::Object(map)
    }
}

#[async_trait]
impl AIProvider for OpenAICompatibleProvider {
    async fn chat(&self, message: &str, system_prompt: Option<&str>) -> Result<String> {
        log::debug!("[{}] Starting chat request", self.provider_type.display_name());
        log::debug!("[{}] Message: {}", self.provider_type.display_name(), message);
        
        let mut messages = Vec::new();

        if let Some(prompt) = system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": prompt
            }));
        }

        messages.push(serde_json::json!({
            "role": "user",
            "content": message
        }));

        let request = OpenAIChatRequest {
            model: self.model.clone(),
            messages,
            tools: None,
        };

        log::debug!("[{}] Model: {}", self.provider_type.display_name(), self.model);
        log::debug!("[{}] Sending request to {}", self.provider_type.display_name(), self.base_url);
        
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[{}] Connection failed: {}", self.provider_type.display_name(), e);
                anyhow!(
                    "Failed to connect to {}: {}",
                    self.provider_type.display_name(),
                    e
                )
            })?;

        log::debug!("[{}] Response status: {}", self.provider_type.display_name(), response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("[{}] API error: {} - {}", self.provider_type.display_name(), status, body);
            return Err(anyhow!(
                "{} API error: {} - {}",
                self.provider_type.display_name(),
                status,
                body
            ));
        }

        let chat_response: OpenAIChatResponse = response
            .json()
            .await
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let content = chat_response
            .choices
            .first()
            .and_then(|c| c.message.content.clone())
            .ok_or_else(|| anyhow!("No response from {}", self.provider_type.display_name()));

        if let Ok(ref c) = content {
            log::debug!("[{}] Response: {}", self.provider_type.display_name(), 
                c.chars().take(200).collect::<String>());
        }
        
        content
    }

    async fn chat_with_tools(&self, request: ChatRequest) -> Result<ChatResponse> {
        log::debug!("[{}] Starting chat_with_tools request", self.provider_type.display_name());
        log::debug!("[{}] Model: {}", self.provider_type.display_name(), self.model);
        log::debug!("[{}] Messages count: {}", self.provider_type.display_name(), request.messages.len());
        log::debug!("[{}] Has tools: {}", self.provider_type.display_name(), request.tools.is_some());
        let mut messages = Vec::new();

        if let Some(ref prompt) = request.system_prompt {
            messages.push(serde_json::json!({
                "role": "system",
                "content": prompt
            }));
        }

        for msg in &request.messages {
            messages.push(self.message_to_json(msg));
        }

        let tools_json = request.tools.map(|tools| {
            tools
                .into_iter()
                .map(|t| {
                    serde_json::json!({
                        "type": t.tool_type,
                        "function": {
                            "name": t.function.name,
                            "description": t.function.description,
                            "parameters": t.function.parameters
                        }
                    })
                })
                .collect()
        });

        let openai_request = OpenAIChatRequest {
            model: self.model.clone(),
            messages,
            tools: tools_json,
        };

        log::debug!("[{}] Sending request to {}", self.provider_type.display_name(), self.base_url);
        log::debug!("[{}] Request messages: {}", self.provider_type.display_name(), 
            serde_json::to_string(&openai_request.messages).unwrap_or_default());
        
        let url = format!("{}/chat/completions", self.base_url);
        let response = self
            .client
            .post(&url)
            .header("Authorization", format!("Bearer {}", self.api_key))
            .header("Content-Type", "application/json")
            .json(&openai_request)
            .send()
            .await
            .map_err(|e| {
                log::error!("[{}] Connection failed: {}", self.provider_type.display_name(), e);
                anyhow!(
                    "Failed to connect to {}: {}",
                    self.provider_type.display_name(),
                    e
                )
            })?;

        log::debug!("[{}] Response status: {}", self.provider_type.display_name(), response.status());

        if !response.status().is_success() {
            let status = response.status();
            let body = response.text().await.unwrap_or_default();
            log::error!("[{}] API error: {} - {}", self.provider_type.display_name(), status, body);
            return Err(anyhow!(
                "{} API error: {} - {}",
                self.provider_type.display_name(),
                status,
                body
            ));
        }

        let raw_response = response.text().await.map_err(|e| anyhow!("Failed to read response: {}", e))?;
        log::debug!("[{}] Raw response (first 1000 chars): {}", self.provider_type.display_name(), 
            raw_response.chars().take(1000).collect::<String>());

        let chat_response: OpenAIChatResponse = serde_json::from_str(&raw_response)
            .map_err(|e| anyhow!("Failed to parse response: {}", e))?;

        let choice = chat_response
            .choices
            .first()
            .ok_or_else(|| anyhow!("No response from {}", self.provider_type.display_name()))?;

        log::debug!("[{}] Has reasoning_content: {}", self.provider_type.display_name(), 
            choice.message.reasoning_content.is_some());

        let tool_calls = if !choice.message.tool_calls.is_empty() {
            log::debug!("[{}] Received {} tool call(s)", self.provider_type.display_name(), choice.message.tool_calls.len());
            for tc in &choice.message.tool_calls {
                log::debug!("[{}] Tool call: {}({})", self.provider_type.display_name(), tc.function.name, tc.function.arguments);
            }
            Some(
                choice
                    .message
                    .tool_calls
                    .iter()
                    .map(|tc| ToolCallRequest {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCallRequest {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect(),
            )
        } else {
            None
        };

        if let Some(ref content) = choice.message.content {
            log::debug!("[{}] Response content: {}", self.provider_type.display_name(), 
                content.chars().take(200).collect::<String>());
        }

        if let Some(ref reasoning) = choice.message.reasoning_content {
            log::debug!("[{}] Reasoning content: {} bytes", self.provider_type.display_name(), reasoning.len());
        }

        Ok(ChatResponse {
            content: choice.message.content.clone(),
            tool_calls,
            reasoning_content: choice.message.reasoning_content.clone(),
        })
    }

    fn provider_name(&self) -> &str {
        self.provider_type.display_name()
    }

    fn supports_tools(&self) -> bool {
        self.provider_type.supports_tools()
    }
}

pub type OpenAIProvider = OpenAICompatibleProvider;
