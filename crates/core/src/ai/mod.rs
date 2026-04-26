pub mod provider;
pub mod ollama;
pub mod openai;
pub mod tools;

pub use provider::{AIProvider, ChatRequest, ChatResponse, ChatMessage, MessageRole};
pub use ollama::OllamaProvider;
pub use openai::{OpenAIProvider, OpenAICompatibleProvider, AIProviderType};
pub use tools::{ToolDefinition, ToolCallRequest, ToolResult, get_tool_definitions};
