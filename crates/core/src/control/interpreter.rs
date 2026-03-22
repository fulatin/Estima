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
    ClearChain,
    ListPlugins {
        filter: Option<String>,
    },
    ShowStatus,
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

IMPORTANT: You MUST use the EXACT parameter names shown in the plugin list. Do NOT guess parameter names!

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
   - Use the EXACT parameter name from the params list (case-sensitive!)
   - Values should be within the specified range

4. Set bypass:
   {"SetBypass": {"bypass": true}}   // Enable bypass (no effects)
   {"SetBypass": {"bypass": false}}  // Disable bypass (effects active)

5. Clear the chain:
   {"ClearChain": {}}

6. Show status:
   {"ShowStatus": {}}

Guidelines:
- Each plugin shows its available parameters with min-max ranges
- You MUST use the exact parameter name shown (e.g., "Decay Time", not "decay")
- When loading a plugin, set appropriate parameters based on the user's request
- For "heavy" or "strong" effects, use higher values (closer to max)
- For "subtle" or "light" effects, use lower values (closer to min)
- Use SetBypass when user wants to "temporarily disable", "turn off effects", or "bypass"
- Use SetBypass with false when user wants to "enable effects" or "turn on effects"

Examples:
User: "Add some reverb"
Response: {
  "commands": [
    {"LoadPlugin": {"uri": "http://calf.sourceforge.net/plugins/Reverb", "position": null}},
    {"SetParameter": {"plugin_id": "@last", "param_name": "Wet Amount", "value": 0.3}}
  ]
}

User: "Heavy distortion"
Response: {
  "commands": [
    {"LoadPlugin": {"uri": "http://calf.sourceforge.net/plugins/Crusher", "position": null}},
    {"SetParameter": {"plugin_id": "@last", "param_name": "Amount", "value": 0.9}}
  ]
}

User: "Bypass the effects"
Response: {
  "commands": [
    {"SetBypass": {"bypass": true}}
  ]
}

User: "Turn effects back on"
Response: {
  "commands": [
    {"SetBypass": {"bypass": false}}
  ]
}

Output ONLY the JSON, no explanation text."#;

pub fn parse_ai_response(response: &str) -> Result<CommandList> {
    let json_str = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();

    let cmd: CommandList = serde_json::from_str(json_str)?;
    Ok(cmd)
}
