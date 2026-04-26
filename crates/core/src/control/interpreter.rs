use anyhow::Result;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Command {
    LoadPlugin {
        uri: String,
        position: Option<usize>,
    },
    RemovePlugin {
        id: String,
    },
    SetParameter {
        plugin_id: String,
        param_name: String,
        value: f32,
    },
    SetBypass {
        bypass: bool,
    },
    ClearChain {
        #[serde(default)]
        #[allow(dead_code)]
        _dummy: (),
    },
    ListPlugins {
        filter: Option<String>,
    },
    ShowStatus {
        #[serde(default)]
        #[allow(dead_code)]
        _dummy: (),
    },
    MovePlugin {
        id: String,
        new_position: usize,
    },
    SavePreset {
        name: String,
    },
    LoadPreset {
        name: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CommandList {
    pub commands: Vec<Command>,
}

impl From<Vec<Command>> for CommandList {
    fn from(commands: Vec<Command>) -> Self {
        Self { commands }
    }
}

pub const SYSTEM_PROMPT: &str = r#"You are an AI assistant controlling an audio effects chain for real-time audio processing.

IMPORTANT INSTRUCTIONS:
1. You have access to tools to search and get plugin information - USE THEM!
2. When the user asks for an effect, FIRST use search_plugins to find available plugins
3. Then use get_plugin_details to see the exact parameter names and ranges
4. Only THEN create the commands with the correct parameter names

Available Tools:
- search_plugins(query): Search for plugins by name, type (reverb, delay, distortion, eq, dynamics, modulation, amp, gain, noise), or keyword
- get_plugin_details(uri): Get detailed parameter info for a specific plugin
- list_plugin_types(): List all available effect categories

Output format (JSON):
{
  "commands": [
    {"LoadPlugin": {"uri": "plugin_uri", "position": null}},
    {"SetParameter": {"plugin_id": "@last", "param_name": "exact_param_name", "value": 0.7}}
  ]
}

Available commands:

1. Load a plugin:
   {"LoadPlugin": {"uri": "plugin_uri", "position": null}}

2. Remove a plugin:
   {"RemovePlugin": {"id": "@last"}}

3. Set a parameter:
   {"SetParameter": {"plugin_id": "@last", "param_name": "exact_name", "value": 0.5}}
   - Use "@last" as plugin_id to reference the most recently loaded plugin
   - Use the EXACT parameter name from get_plugin_details (case-sensitive!)
   - Values should be within the specified range

4. Set bypass:
   {"SetBypass": {"bypass": true}}   // Enable bypass (no effects)
   {"SetBypass": {"bypass": false}}  // Disable bypass (effects active)

5. Clear the chain:
   {"ClearChain": {}}

6. Show status:
   {"ShowStatus": {}}

Guidelines:
- ALWAYS use tools to find the best plugin and verify parameter names
- For "heavy" or "strong" effects, use higher values (closer to max)
- For "subtle" or "light" effects, use lower values (closer to min)
- Use SetBypass when user wants to "temporarily disable" or "bypass"
- Use SetBypass with false when user wants to "enable effects"

Output ONLY the JSON, no explanation text."#;

pub const SYSTEM_PROMPT_WITH_CONTEXT: &str = r#"You are an AI assistant controlling an audio effects chain for real-time audio processing.

IMMEDIATELY use the provided plugin context to select the most appropriate plugin and parameters.

Output format (JSON):
{
  "commands": [
    {"LoadPlugin": {"uri": "plugin_uri", "position": null}},
    {"SetParameter": {"plugin_id": "@last", "param_name": "exact_param_name", "value": 0.7}}
  ]
}

Available commands:

1. Load a plugin:
   {"LoadPlugin": {"uri": "plugin_uri", "position": null}}

2. Remove a plugin:
   {"RemovePlugin": {"id": "@last"}}

3. Set a parameter:
   {"SetParameter": {"plugin_id": "@last", "param_name": "exact_name", "value": 0.5}}
   - Use "@last" as plugin_id to reference the most recently loaded plugin
   - Values should be within the specified range

4. Set bypass:
   {"SetBypass": {"bypass": true}}
   {"SetBypass": {"bypass": false}}

5. Clear the chain:
   {"ClearChain": {}}

6. Show status:
   {"ShowStatus": {}}

Guidelines:
- Use the EXACT parameter names shown (case-sensitive!)
- For "heavy" effects, use higher values
- For "subtle" effects, use lower values

Output ONLY the JSON, no explanation text."#;

pub fn parse_ai_response(response: &str) -> Result<CommandList> {
    log::debug!("Parsing AI response ({} bytes)", response.len());
    log::debug!("Raw response: {}", response.chars().take(500).collect::<String>());

    let trimmed = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let json_str = if let Some(start) = trimmed.find("{") {
        let from_start = &trimmed[start..];
        let mut brace_count = 0;
        let mut end = from_start.len();
        for (i, c) in from_start.char_indices() {
            match c {
                '{' => brace_count += 1,
                '}' => {
                    brace_count -= 1;
                    if brace_count == 0 {
                        end = i + 1;
                        break;
                    }
                }
                _ => {}
            }
        }
        &from_start[..end]
    } else {
        trimmed
    };

    let cmd: CommandList = serde_json::from_str(json_str)?;
    log::debug!("Parsed {} command(s)", cmd.commands.len());
    for (i, c) in cmd.commands.iter().enumerate() {
        log::debug!("Command {}: {:?}", i, c);
    }
    Ok(cmd)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_clean_json() {
        let input = r#"{"commands":[{"LoadPlugin":{"uri":"http://example.org/plugin","position":null}}]}"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        matches!(&result.commands[0], Command::LoadPlugin { .. });
    }

    #[test]
    fn parse_with_json_fence() {
        let input = r#"```json
{"commands":[{"SetBypass":{"bypass":true}}]}
```"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        match &result.commands[0] {
            Command::SetBypass { bypass } => assert!(bypass),
            _ => panic!("Expected SetBypass"),
        }
    }

    #[test]
    fn parse_with_plain_fence() {
        let input = r#"```
{"commands":[{"SetParameter":{"plugin_id":"@last","param_name":"mix","value":0.5}}]}
```"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        match &result.commands[0] {
            Command::SetParameter { param_name, .. } => assert_eq!(param_name, "mix"),
            _ => panic!("Expected SetParameter"),
        }
    }

    #[test]
    fn parse_with_whitespace() {
        let input = r#"  
  {"commands":[{"SetBypass":{"bypass":false}}]}  
  "#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        match &result.commands[0] {
            Command::SetBypass { bypass } => assert!(!bypass),
            _ => panic!("Expected SetBypass"),
        }
    }

    #[test]
    fn parse_remove_plugin() {
        let input = r#"{"commands":[{"RemovePlugin":{"id":"some-id"}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::RemovePlugin { id } => assert_eq!(id, "some-id"),
            _ => panic!("Expected RemovePlugin"),
        }
    }

    #[test]
    fn parse_set_parameter() {
        let input = r#"{"commands":[{"SetParameter":{"plugin_id":"@last","param_name":"gain","value":0.75}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::SetParameter {
                plugin_id,
                param_name,
                value,
            } => {
                assert_eq!(plugin_id, "@last");
                assert_eq!(param_name, "gain");
                assert_eq!(*value, 0.75);
            }
            _ => panic!("Expected SetParameter"),
        }
    }

    #[test]
    fn parse_list_plugins() {
        let input = r#"{"commands":[{"ListPlugins":{"filter":"reverb"}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::ListPlugins { filter } => assert_eq!(filter.as_deref(), Some("reverb")),
            _ => panic!("Expected ListPlugins"),
        }
    }

    #[test]
    fn parse_move_plugin() {
        let input = r#"{"commands":[{"MovePlugin":{"id":"abc","new_position":2}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::MovePlugin { id, new_position } => {
                assert_eq!(id, "abc");
                assert_eq!(*new_position, 2);
            }
            _ => panic!("Expected MovePlugin"),
        }
    }

    #[test]
    fn parse_save_preset() {
        let input = r#"{"commands":[{"SavePreset":{"name":"my-preset"}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::SavePreset { name } => assert_eq!(name, "my-preset"),
            _ => panic!("Expected SavePreset"),
        }
    }

    #[test]
    fn parse_load_preset() {
        let input = r#"{"commands":[{"LoadPreset":{"name":"my-preset"}}]}"#;
        let result = parse_ai_response(input).unwrap();
        match &result.commands[0] {
            Command::LoadPreset { name } => assert_eq!(name, "my-preset"),
            _ => panic!("Expected LoadPreset"),
        }
    }

    #[test]
    fn parse_multiple_commands() {
        let input = r#"{"commands":[{"LoadPlugin":{"uri":"http://example.org/reverb","position":null}},{"SetParameter":{"plugin_id":"@last","param_name":"mix","value":0.5}}]}"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 2);
    }

    #[test]
    fn parse_invalid_json() {
        let input = "not json at all";
        assert!(parse_ai_response(input).is_err());
    }

    #[test]
    fn parse_empty_commands() {
        let input = r#"{"commands":[]}"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 0);
    }

    #[test]
    fn parse_show_status_with_empty_object() {
        let input = r#"{"commands": [{"ShowStatus": {}}]}"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        matches!(&result.commands[0], Command::ShowStatus { .. });
    }

    #[test]
    fn parse_clear_chain_with_empty_object() {
        let input = r#"{"commands": [{"ClearChain": {}}]}"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        matches!(&result.commands[0], Command::ClearChain { .. });
    }

    #[test]
    fn parse_json_with_prefix_text() {
        let input = r#"Here's a heavy metal chain for you!

{
  "commands": [
    {"LoadPlugin": {"uri": "http://example.org/plugin", "position": null}},
    {"SetParameter": {"plugin_id": "@last", "param_name": "DRIVE", "value": 20.0}}
  ]
}

Hope you like it!"#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 2);
        matches!(&result.commands[0], Command::LoadPlugin { .. });
    }

    #[test]
    fn parse_json_with_prefix_and_suffix_text() {
        let input = r#"Sure, here's what I'll do:

{"commands":[{"ShowStatus":{}}]}

That should show the current status."#;
        let result = parse_ai_response(input).unwrap();
        assert_eq!(result.commands.len(), 1);
        matches!(&result.commands[0], Command::ShowStatus { .. });
    }
}
