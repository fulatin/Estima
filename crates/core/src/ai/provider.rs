use async_trait::async_trait;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use super::tools::{ToolCallRequest, ToolDefinition};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChatMessage {
    pub role: MessageRole,
    #[serde(skip_serializing_if = "String::is_empty")]
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCallRequest>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
}

#[derive(Debug, Clone)]
pub struct ChatRequest {
    pub messages: Vec<ChatMessage>,
    pub tools: Option<Vec<ToolDefinition>>,
    pub system_prompt: Option<String>,
}

impl ChatRequest {
    pub fn new() -> Self {
        Self {
            messages: Vec::new(),
            tools: None,
            system_prompt: None,
        }
    }

    pub fn with_system_prompt(mut self, prompt: &str) -> Self {
        self.system_prompt = Some(prompt.to_string());
        self
    }

    pub fn with_tools(mut self, tools: Vec<ToolDefinition>) -> Self {
        self.tools = Some(tools);
        self
    }

    pub fn add_message(mut self, role: MessageRole, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role,
            content: content.to_string(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
        });
        self
    }

    pub fn add_full_message(mut self, msg: ChatMessage) -> Self {
        self.messages.push(msg);
        self
    }

    pub fn add_assistant_with_tools(mut self, content: &str, tool_calls: Vec<ToolCallRequest>) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: content.to_string(),
            name: None,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            reasoning_content: None,
        });
        self
    }

    pub fn add_assistant_with_tools_and_reasoning(
        mut self,
        content: &str,
        tool_calls: Vec<ToolCallRequest>,
        reasoning_content: Option<String>,
    ) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::Assistant,
            content: content.to_string(),
            name: None,
            tool_call_id: None,
            tool_calls: Some(tool_calls),
            reasoning_content,
        });
        self
    }

    pub fn add_tool_result(mut self, tool_call_id: &str, name: &str, content: &str) -> Self {
        self.messages.push(ChatMessage {
            role: MessageRole::Tool,
            content: content.to_string(),
            name: Some(name.to_string()),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
            reasoning_content: None,
        });
        self
    }
}

#[derive(Debug, Clone)]
pub struct ChatResponse {
    pub content: Option<String>,
    pub tool_calls: Option<Vec<ToolCallRequest>>,
    pub reasoning_content: Option<String>,
}

impl ChatResponse {
    pub fn has_tool_calls(&self) -> bool {
        self.tool_calls.is_some() && !self.tool_calls.as_ref().unwrap().is_empty()
    }
}

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn chat(&self, message: &str, system_prompt: Option<&str>) -> Result<String>;
    async fn chat_with_tools(&self, request: ChatRequest) -> Result<ChatResponse>;
    fn provider_name(&self) -> &str;
    fn supports_tools(&self) -> bool {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::tools::FunctionCallRequest;

    #[test]
    fn chat_request_builder_empty() {
        let req = ChatRequest::new();
        assert!(req.messages.is_empty());
        assert!(req.tools.is_none());
        assert!(req.system_prompt.is_none());
    }

    #[test]
    fn chat_request_with_system_prompt() {
        let req = ChatRequest::new().with_system_prompt("You are helpful");
        assert_eq!(req.system_prompt.as_deref(), Some("You are helpful"));
    }

    #[test]
    fn chat_request_with_tools() {
        let tools = crate::ai::tools::get_tool_definitions();
        let tools_len = tools.len();
        let req = ChatRequest::new().with_tools(tools);
        assert!(req.tools.is_some());
        assert_eq!(req.tools.unwrap().len(), tools_len);
    }

    #[test]
    fn chat_request_add_message() {
        let req = ChatRequest::new().add_message(MessageRole::User, "Hello");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, MessageRole::User);
        assert_eq!(req.messages[0].content, "Hello");
    }

    #[test]
    fn chat_request_add_assistant_with_tools() {
        let tool_call = ToolCallRequest {
            id: "call_1".to_string(),
            call_type: "function".to_string(),
            function: FunctionCallRequest {
                name: "search".to_string(),
                arguments: "{}".to_string(),
            },
        };
        let req =
            ChatRequest::new().add_assistant_with_tools("Using tools", vec![tool_call.clone()]);
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].content, "Using tools");
        assert!(req.messages[0].tool_calls.is_some());
    }

    #[test]
    fn chat_request_add_tool_result() {
        let req = ChatRequest::new().add_tool_result("call_1", "search", "result data");
        assert_eq!(req.messages.len(), 1);
        assert_eq!(req.messages[0].role, MessageRole::Tool);
        assert_eq!(req.messages[0].name.as_deref(), Some("search"));
        assert_eq!(req.messages[0].tool_call_id.as_deref(), Some("call_1"));
    }

    #[test]
    fn chat_response_has_tool_calls_true() {
        let resp = ChatResponse {
            content: None,
            tool_calls: Some(vec![ToolCallRequest {
                id: "t1".to_string(),
                call_type: "function".to_string(),
                function: FunctionCallRequest {
                    name: "f".to_string(),
                    arguments: "{}".to_string(),
                },
            }]),
            reasoning_content: None,
        };
        assert!(resp.has_tool_calls());
    }

    #[test]
    fn chat_response_has_tool_calls_false_empty() {
        let resp = ChatResponse {
            content: Some("text".to_string()),
            tool_calls: Some(vec![]),
            reasoning_content: None,
        };
        assert!(!resp.has_tool_calls());
    }

    #[test]
    fn chat_response_has_tool_calls_false_none() {
        let resp = ChatResponse {
            content: Some("text".to_string()),
            tool_calls: None,
            reasoning_content: None,
        };
        assert!(!resp.has_tool_calls());
    }

    #[test]
    fn chat_message_serialization_skips_none_fields() {
        let msg = ChatMessage {
            role: MessageRole::User,
            content: "hello".to_string(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("name"));
        assert!(!json.contains("tool_call_id"));
        assert!(!json.contains("tool_calls"));
        assert!(!json.contains("reasoning_content"));
    }

    #[test]
    fn chat_message_serialization_skips_empty_content() {
        let msg = ChatMessage {
            role: MessageRole::Assistant,
            content: String::new(),
            name: None,
            tool_call_id: None,
            tool_calls: None,
            reasoning_content: None,
        };
        let json = serde_json::to_string(&msg).unwrap();
        assert!(!json.contains("content"));
    }

    #[test]
    fn message_role_serialization() {
        let json = serde_json::to_string(&MessageRole::System).unwrap();
        assert_eq!(json, "\"System\"");
        let json = serde_json::to_string(&MessageRole::User).unwrap();
        assert_eq!(json, "\"User\"");
        let json = serde_json::to_string(&MessageRole::Assistant).unwrap();
        assert_eq!(json, "\"Assistant\"");
        let json = serde_json::to_string(&MessageRole::Tool).unwrap();
        assert_eq!(json, "\"Tool\"");
    }
}
