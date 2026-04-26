use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum MessageRole {
    System,
    User,
    Assistant,
    Tool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Message {
    pub role: MessageRole,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_call_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tool_calls: Option<Vec<ToolCall>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reasoning_content: Option<String>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCall {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCall,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCall {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Conversation {
    pub id: String,
    pub messages: Vec<Message>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub created_at: DateTime<Utc>,
    #[serde(with = "chrono::serde::ts_seconds")]
    pub updated_at: DateTime<Utc>,
}

impl Default for Conversation {
    fn default() -> Self {
        Self::new()
    }
}

impl Conversation {
    pub fn new() -> Self {
        let now = Utc::now();
        Self {
            id: uuid::Uuid::new_v4().to_string(),
            messages: Vec::new(),
            created_at: now,
            updated_at: now,
        }
    }

    pub fn add_message(&mut self, role: MessageRole, content: &str) {
        self.add_message_with_meta(role, content, None, None, None, None);
    }

    pub fn add_message_with_meta(
        &mut self,
        role: MessageRole,
        content: &str,
        name: Option<String>,
        tool_call_id: Option<String>,
        tool_calls: Option<Vec<ToolCall>>,
        reasoning_content: Option<String>,
    ) {
        self.messages.push(Message {
            role,
            content: content.to_string(),
            name,
            tool_call_id,
            tool_calls,
            reasoning_content,
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn add_tool_result(&mut self, tool_call_id: &str, name: &str, content: &str) {
        self.messages.push(Message {
            role: MessageRole::Tool,
            content: content.to_string(),
            name: Some(name.to_string()),
            tool_call_id: Some(tool_call_id.to_string()),
            tool_calls: None,
            reasoning_content: None,
            timestamp: Utc::now(),
        });
        self.updated_at = Utc::now();
    }

    pub fn clear(&mut self) {
        self.messages.clear();
        self.updated_at = Utc::now();
    }

    pub fn to_openai_messages(&self) -> Vec<serde_json::Value> {
        self.messages
            .iter()
            .map(|m| {
                let mut msg = serde_json::Map::new();
                msg.insert("role".to_string(), serde_json::json!(m.role));
                if !m.content.is_empty() {
                    msg.insert("content".to_string(), serde_json::json!(&m.content));
                }
                if let Some(ref name) = m.name {
                    msg.insert("name".to_string(), serde_json::json!(name));
                }
                if let Some(ref id) = m.tool_call_id {
                    msg.insert("tool_call_id".to_string(), serde_json::json!(id));
                }
                if let Some(ref calls) = m.tool_calls {
                    msg.insert("tool_calls".to_string(), serde_json::json!(calls));
                }
                if let Some(ref reasoning) = m.reasoning_content {
                    msg.insert("reasoning_content".to_string(), serde_json::json!(reasoning));
                }
                serde_json::Value::Object(msg)
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn new_conversation_has_defaults() {
        let conv = Conversation::new();
        assert!(!conv.id.is_empty());
        assert!(conv.messages.is_empty());
        assert!(conv.created_at <= Utc::now());
        assert!(conv.updated_at <= Utc::now());
    }

    #[test]
    fn add_message_appends() {
        let mut conv = Conversation::new();
        conv.add_message(MessageRole::User, "Hello");
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].role, MessageRole::User);
        assert_eq!(conv.messages[0].content, "Hello");
        assert!(conv.messages[0].timestamp <= Utc::now());
    }

    #[test]
    fn add_message_updates_timestamp() {
        let mut conv = Conversation::new();
        let before = conv.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        conv.add_message(MessageRole::System, "system msg");
        assert!(conv.updated_at > before);
    }

    #[test]
    fn add_message_with_meta() {
        let mut conv = Conversation::new();
        conv.add_message_with_meta(
            MessageRole::Assistant,
            "response",
            Some("assistant".to_string()),
            None,
            None,
            None,
        );
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].name.as_deref(), Some("assistant"));
    }

    #[test]
    fn add_tool_result() {
        let mut conv = Conversation::new();
        conv.add_tool_result("call_1", "search_plugins", "result content");
        assert_eq!(conv.messages.len(), 1);
        assert_eq!(conv.messages[0].role, MessageRole::Tool);
        assert_eq!(conv.messages[0].tool_call_id.as_deref(), Some("call_1"));
        assert_eq!(
            conv.messages[0].name.as_deref(),
            Some("search_plugins")
        );
        assert_eq!(conv.messages[0].content, "result content");
    }

    #[test]
    fn clear_empties_messages() {
        let mut conv = Conversation::new();
        conv.add_message(MessageRole::User, "msg1");
        conv.add_message(MessageRole::Assistant, "msg2");
        assert_eq!(conv.messages.len(), 2);

        let before = conv.updated_at;
        std::thread::sleep(std::time::Duration::from_millis(10));
        conv.clear();
        assert!(conv.messages.is_empty());
        assert!(conv.updated_at > before);
    }

    #[test]
    fn to_openai_messages_basic() {
        let mut conv = Conversation::new();
        conv.add_message(MessageRole::User, "Hello");
        conv.add_message(MessageRole::Assistant, "Hi there");
        let msgs = conv.to_openai_messages();
        assert_eq!(msgs.len(), 2);
        assert_eq!(msgs[0]["role"], "User");
        assert_eq!(msgs[0]["content"], "Hello");
        assert_eq!(msgs[1]["role"], "Assistant");
        assert_eq!(msgs[1]["content"], "Hi there");
    }

    #[test]
    fn to_openai_messages_skips_empty_content() {
        let mut conv = Conversation::new();
        conv.add_message(MessageRole::Assistant, "");
        let msgs = conv.to_openai_messages();
        assert_eq!(msgs.len(), 1);
        assert!(msgs[0].get("content").is_none());
    }

    #[test]
    fn conversation_json_roundtrip() {
        let mut conv = Conversation::new();
        conv.add_message(MessageRole::User, "test message");
        let json = serde_json::to_string(&conv).unwrap();
        let decoded: Conversation = serde_json::from_str(&json).unwrap();
        assert_eq!(decoded.id, conv.id);
        assert_eq!(decoded.messages.len(), 1);
        assert_eq!(decoded.messages[0].role, MessageRole::User);
        assert_eq!(decoded.messages[0].content, "test message");
    }

    #[test]
    fn message_role_display_and_parse() {
        let json = serde_json::to_string(&MessageRole::System).unwrap();
        assert_eq!(json, "\"System\"");
        let role: MessageRole = serde_json::from_str("\"User\"").unwrap();
        assert_eq!(role, MessageRole::User);
    }
}
