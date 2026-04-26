use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const TOOL_SEARCH_PLUGINS: &str = "search_plugins";
pub const TOOL_GET_PLUGIN_DETAILS: &str = "get_plugin_details";
pub const TOOL_LIST_PLUGIN_TYPES: &str = "list_plugin_types";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolDefinition {
    #[serde(rename = "type")]
    pub tool_type: String,
    pub function: FunctionDefinition,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionDefinition {
    pub name: String,
    pub description: String,
    pub parameters: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ToolCallRequest {
    pub id: String,
    #[serde(rename = "type")]
    pub call_type: String,
    pub function: FunctionCallRequest,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FunctionCallRequest {
    pub name: String,
    pub arguments: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolResult {
    pub tool_call_id: String,
    pub content: String,
}

pub fn get_tool_definitions() -> Vec<ToolDefinition> {
    vec![
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_SEARCH_PLUGINS.to_string(),
                description: "Search for audio plugins by name, type, or keyword. Use this when the user mentions a specific effect type or asks what plugins are available.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "query": {
                            "type": "string",
                            "description": "Search query - can be a plugin name, type (reverb, delay, distortion, eq, dynamics, modulation, amp, gain, noise), or keyword"
                        }
                    },
                    "required": ["query"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_GET_PLUGIN_DETAILS.to_string(),
                description: "Get detailed information about a specific plugin including all parameters with their ranges. Use this after search_plugins to get full parameter info for plugin selection.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {
                        "uri": {
                            "type": "string",
                            "description": "The plugin URI (exact URI from search results)"
                        }
                    },
                    "required": ["uri"]
                }),
            },
        },
        ToolDefinition {
            tool_type: "function".to_string(),
            function: FunctionDefinition {
                name: TOOL_LIST_PLUGIN_TYPES.to_string(),
                description: "List all available plugin types/categories in the system. Use this when the user wants to know what types of effects are available.".to_string(),
                parameters: serde_json::json!({
                    "type": "object",
                    "properties": {},
                    "required": []
                }),
            },
        },
    ]
}

pub fn parse_tool_arguments<T: for<'de> Deserialize<'de>>(args: &str) -> Result<T, serde_json::Error> {
    serde_json::from_str(args)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tool_definitions_has_three_tools() {
        let tools = get_tool_definitions();
        assert_eq!(tools.len(), 3);
    }

    #[test]
    fn tool_definitions_names() {
        let tools = get_tool_definitions();
        let names: Vec<&str> = tools.iter().map(|t| t.function.name.as_str()).collect();
        assert!(names.contains(&"search_plugins"));
        assert!(names.contains(&"get_plugin_details"));
        assert!(names.contains(&"list_plugin_types"));
    }

    #[test]
    fn tool_definitions_all_have_type_function() {
        let tools = get_tool_definitions();
        for tool in &tools {
            assert_eq!(tool.tool_type, "function");
        }
    }

    #[test]
    fn parse_search_plugins_args_valid() {
        let args: SearchPluginsArgs = parse_tool_arguments(r#"{"query":"reverb"}"#).unwrap();
        assert_eq!(args.query, "reverb");
    }

    #[test]
    fn parse_get_plugin_details_args_valid() {
        let args: GetPluginDetailsArgs =
            parse_tool_arguments(r#"{"uri":"http://example.org/plugin"}"#).unwrap();
        assert_eq!(args.uri, "http://example.org/plugin");
    }

    #[test]
    fn parse_tool_arguments_invalid_json() {
        let result: Result<SearchPluginsArgs, _> = parse_tool_arguments("not json");
        assert!(result.is_err());
    }

    #[test]
    fn parse_tool_arguments_missing_field() {
        let result: Result<SearchPluginsArgs, _> = parse_tool_arguments(r#"{"wrong":"value"}"#);
        assert!(result.is_err());
    }

    #[test]
    fn tool_call_request_deserialization() {
        let json = r#"{"id":"call_1","type":"function","function":{"name":"search","arguments":"{\"query\":\"delay\"}"}}"#;
        let req: ToolCallRequest = serde_json::from_str(json).unwrap();
        assert_eq!(req.id, "call_1");
        assert_eq!(req.call_type, "function");
        assert_eq!(req.function.name, "search");
    }
}

#[derive(Debug, Clone, Deserialize)]
pub struct SearchPluginsArgs {
    pub query: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetPluginDetailsArgs {
    pub uri: String,
}
