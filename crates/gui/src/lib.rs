use anyhow::Result;
use estima_core::ai::{
    AIProvider, AIProviderType, ChatMessage, ChatRequest, MessageRole, OpenAICompatibleProvider,
    ToolCallRequest, get_tool_definitions,
    tools::{
        FunctionCallRequest, GetPluginDetailsArgs, SearchPluginsArgs, TOOL_GET_PLUGIN_DETAILS,
        TOOL_LIST_PLUGIN_TYPES, TOOL_SEARCH_PLUGINS, parse_tool_arguments,
    },
};
use estima_core::audio::{AudioState as JackAudioState, JackEngine, LV2ExternalUIManager, PluginChain};
use estima_core::control::{
    CommandList, SYSTEM_PROMPT, SYSTEM_PROMPT_WITH_CONTEXT, parse_ai_response,
};
use estima_core::memory::{
    Conversation, FunctionCall, MemoryStorage, MessageRole as MemoryRole, ToolCall,
};
use estima_core::{AIConfig, AppConfig};
use serde::{Deserialize, Serialize};
use std::sync::{Arc, Mutex};
use tauri::{Emitter, Manager, State};

#[derive(Debug, Clone)]
pub struct AIResponse {
    pub content: String,
    pub reasoning_content: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct PluginInfo {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub plugin_type: String,
    pub bypass: bool,
    pub has_ui: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ParameterInfo {
    pub name: String,
    pub symbol: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub current: f32,
    pub port_index: u32,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ChainStatus {
    pub plugins: Vec<PluginInfo>,
    pub bypass: bool,
    pub last_loaded_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct HistoryMessage {
    pub role: String,
    pub content: String,
}

pub struct AudioState {
    pub chain: Arc<Mutex<PluginChain>>,
    pub last_loaded_id: Arc<Mutex<Option<String>>>,
    pub sample_rate: f64,
    pub started: bool,
    pub ui_manager: Arc<Mutex<Option<LV2ExternalUIManager>>>,
    pub app_handle: Arc<Mutex<Option<tauri::AppHandle>>>,
}

pub struct AppState {
    pub memory_storage: MemoryStorage,
    pub conversation: Arc<Mutex<Conversation>>,
    pub config: Arc<Mutex<AppConfig>>,
}

fn execute_tool(tool_name: &str, arguments: &str, chain: &PluginChain) -> Result<String> {
    log::debug!("Executing tool: {} with args: {}", tool_name, arguments);
    let result = match tool_name {
        TOOL_SEARCH_PLUGINS => {
            let args: SearchPluginsArgs = parse_tool_arguments(arguments)?;
            let results = chain.search_plugins(&args.query);
            let output: Vec<String> = results
                .iter()
                .map(|p| format!("- {} ({})\n  URI: {}", p.name, p.plugin_type, p.uri))
                .collect();
            if output.is_empty() {
                Ok("No plugins found matching the query.".to_string())
            } else {
                Ok(format!(
                    "Found {} plugin(s):\n{}",
                    results.len(),
                    output.join("\n")
                ))
            }
        }
        TOOL_GET_PLUGIN_DETAILS => {
            let args: GetPluginDetailsArgs = parse_tool_arguments(arguments)?;
            if let Some(params) = chain.get_plugin_parameters(&args.uri) {
                let plugin_info = chain
                    .search_plugins(&args.uri)
                    .first()
                    .map(|p| format!("{} ({})", p.name, p.plugin_type))
                    .unwrap_or_else(|| "Unknown Plugin".to_string());

                let params_str: Vec<String> = params
                    .iter()
                    .map(|p| {
                        format!(
                            "  - {}: {:.2} - {:.2} (default: {:.2})",
                            p.name, p.min, p.max, p.default
                        )
                    })
                    .collect();

                Ok(format!(
                    "Plugin: {}\nURI: {}\nParameters:\n{}",
                    plugin_info,
                    args.uri,
                    params_str.join("\n")
                ))
            } else {
                Ok(format!("Plugin not found: {}", args.uri))
            }
        }
        TOOL_LIST_PLUGIN_TYPES => {
            let plugins = chain.list_available_plugins();
            let mut types: std::collections::HashSet<&str> = std::collections::HashSet::new();
            for p in plugins {
                types.insert(p.plugin_type.as_str());
            }
            let mut type_list: Vec<&str> = types.into_iter().collect();
            type_list.sort();
            Ok(format!("Available plugin types: {}", type_list.join(", ")))
        }
        _ => Ok(format!("Unknown tool: {}", tool_name)),
    };
    match &result {
        Ok(r) => log::debug!("Tool {} result: {}", tool_name, r.chars().take(200).collect::<String>()),
        Err(e) => log::debug!("Tool {} error: {}", tool_name, e),
    }
    result
}

#[tauri::command]
fn list_plugins(
    filter: Option<String>,
    state: State<AudioState>,
) -> Result<Vec<PluginInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let plugins: Vec<PluginInfo> = if let Some(f) = filter {
        chain
            .search_plugins(&f)
            .into_iter()
            .map(|p| PluginInfo {
                id: p.uri.clone(),
                uri: p.uri.clone(),
                name: p.name.clone(),
                plugin_type: p.plugin_type.clone(),
                bypass: false,
                has_ui: p.has_ui,
            })
            .collect()
    } else {
        chain
            .list_available_plugins()
            .iter()
            .map(|p| PluginInfo {
                id: p.uri.clone(),
                uri: p.uri.clone(),
                name: p.name.clone(),
                plugin_type: p.plugin_type.clone(),
                bypass: false,
                has_ui: p.has_ui,
            })
            .collect()
    };
    
    let ui_count = plugins.iter().filter(|p| p.has_ui).count();
    log::info!("list_plugins: {} total, {} with UI", plugins.len(), ui_count);
    if let Some(p) = plugins.iter().find(|p| p.has_ui) {
        log::info!("First plugin with UI: {} (has_ui={})", p.name, p.has_ui);
    }
    
    Ok(plugins)
}

#[tauri::command]
fn load_plugin(uri: String, state: State<AudioState>) -> Result<PluginInfo, String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    let (id, _params) = chain
        .load_plugin(&uri, state.sample_rate)
        .map_err(|e| e.to_string())?;

    let mut last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    *last_id = Some(id.clone());

    let plugin = chain
        .get_active_plugins()
        .iter()
        .find(|p| p.id == id)
        .ok_or("Plugin not found after loading")?;

    Ok(PluginInfo {
        id,
        uri,
        name: plugin.info.name.clone(),
        plugin_type: plugin.info.plugin_type.clone(),
        bypass: plugin.bypass,
        has_ui: plugin.info.has_ui,
    })
}

#[tauri::command]
fn remove_plugin(id: String, state: State<AudioState>) -> Result<(), String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    chain.remove_plugin(&id).map_err(|e| e.to_string())?;

    let mut last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    if last_id.as_ref() == Some(&id) {
        *last_id = None;
    }
    Ok(())
}

#[tauri::command]
fn toggle_plugin_bypass(id: String, state: State<AudioState>) -> Result<bool, String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    chain.toggle_plugin_bypass(&id).map_err(|e| e.to_string())
}

#[tauri::command]
fn move_plugin(id: String, direction: i32, state: State<AudioState>) -> Result<(), String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    chain.move_plugin(&id, direction).map_err(|e| e.to_string())
}

#[tauri::command]
fn get_plugin_parameters(
    uri: String,
    state: State<AudioState>,
) -> Result<Vec<ParameterInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let params = chain
        .get_plugin_parameters(&uri)
        .ok_or("Plugin not found")?;

    Ok(params
        .into_iter()
        .map(|p| ParameterInfo {
            name: p.name,
            symbol: p.symbol,
            default: p.default,
            min: p.min,
            max: p.max,
            current: p.default,
            port_index: p.port_index,
        })
        .collect())
}

#[tauri::command]
fn get_active_plugin_parameters(
    plugin_id: String,
    state: State<AudioState>,
) -> Result<Vec<ParameterInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let plugin = chain
        .get_plugin_by_id(&plugin_id)
        .ok_or("Plugin not found")?;

    Ok(plugin
        .parameters
        .iter()
        .zip(plugin.parameter_values.iter())
        .map(|(p, (_, value))| ParameterInfo {
            name: p.name.clone(),
            symbol: p.symbol.clone(),
            default: p.default,
            min: p.min,
            max: p.max,
            current: *value,
            port_index: p.port_index,
        })
        .collect())
}

#[tauri::command]
fn set_parameter(
    plugin_id: String,
    param_name: String,
    value: f32,
    state: State<AudioState>,
) -> Result<(), String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;

    let target_id = if plugin_id == "@last" {
        state
            .last_loaded_id
            .lock()
            .map_err(|e| e.to_string())?
            .clone()
            .ok_or("No plugin has been loaded yet")?
    } else {
        plugin_id
    };

    chain
        .set_parameter(&target_id, &param_name, value)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_chain_status(state: State<AudioState>) -> Result<ChainStatus, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;

    let plugins: Vec<PluginInfo> = chain
        .get_active_plugins()
        .iter()
        .map(|p| PluginInfo {
            id: p.id.clone(),
            uri: p.info.uri.clone(),
            name: p.info.name.clone(),
            plugin_type: p.info.plugin_type.clone(),
            bypass: p.bypass,
            has_ui: p.info.has_ui,
        })
        .collect();

    for p in &plugins {
        log::info!("Active plugin: {} has_ui={}", p.name, p.has_ui);
    }

    Ok(ChainStatus {
        plugins,
        bypass: chain.bypass(),
        last_loaded_id: last_id.clone(),
    })
}

#[tauri::command]
fn toggle_bypass(state: State<AudioState>) -> Result<bool, String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    let new_state = chain.toggle_bypass();
    Ok(new_state)
}

#[tauri::command]
fn save_preset(name: String, state: State<AudioState>) -> Result<String, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let path = chain.save_config(&name, None).map_err(|e| e.to_string())?;
    Ok(path.to_string_lossy().to_string())
}

#[tauri::command]
fn load_preset(name: String, state: State<AudioState>) -> Result<ChainStatus, String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    let path = format!("{}.estima.json", name);
    let _config = chain
        .load_config(&path, state.sample_rate)
        .map_err(|e| e.to_string())?;

    let mut last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    if let Some(last) = chain.get_active_plugins().last() {
        *last_id = Some(last.id.clone());
    }

    let plugins: Vec<PluginInfo> = chain
        .get_active_plugins()
        .iter()
        .map(|p| PluginInfo {
            id: p.id.clone(),
            uri: p.info.uri.clone(),
            name: p.info.name.clone(),
            plugin_type: p.info.plugin_type.clone(),
            bypass: p.bypass,
            has_ui: p.info.has_ui,
        })
        .collect();

    Ok(ChainStatus {
        plugins,
        bypass: chain.bypass(),
        last_loaded_id: last_id.clone(),
    })
}

#[tauri::command]
fn list_presets() -> Result<Vec<String>, String> {
    let presets = PluginChain::list_presets().map_err(|e| e.to_string())?;
    Ok(presets.into_iter().map(|(name, _)| name).collect())
}

#[tauri::command]
fn get_history(state: State<AppState>) -> Result<Vec<HistoryMessage>, String> {
    let conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    Ok(conversation
        .messages
        .iter()
        .map(|m| HistoryMessage {
            role: match m.role {
                MemoryRole::System => "system",
                MemoryRole::User => "user",
                MemoryRole::Assistant => "assistant",
                MemoryRole::Tool => "tool",
            }
            .to_string(),
            content: m.content.clone(),
        })
        .collect())
}

#[tauri::command]
fn clear_history(state: State<AppState>) -> Result<(), String> {
    let mut conversation = state.conversation.lock().map_err(|e| e.to_string())?;
    conversation.clear();
    state
        .memory_storage
        .save(&conversation)
        .map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_config(state: State<AppState>) -> Result<AppConfig, String> {
    let config = state.config.lock().map_err(|e| e.to_string())?;
    Ok(config.clone())
}

#[tauri::command]
fn save_config(state: State<AppState>, config: AppConfig) -> Result<(), String> {
    config.save().map_err(|e| e.to_string())?;
    let mut current = state.config.lock().map_err(|e| e.to_string())?;
    *current = config;
    Ok(())
}

#[tauri::command]
async fn test_ai_connection(config: AIConfig) -> Result<String, String> {
    let provider: Box<dyn AIProvider> = match config.provider.as_str() {
        "siliconflow" => Box::new(OpenAICompatibleProvider::new(
            AIProviderType::SiliconFlow,
            &config.api_key,
            config.model.as_deref(),
        )),
        "deepseek" => Box::new(OpenAICompatibleProvider::new(
            AIProviderType::DeepSeek,
            &config.api_key,
            config.model.as_deref(),
        )),
        "openai" => Box::new(OpenAICompatibleProvider::new(
            AIProviderType::OpenAI,
            &config.api_key,
            config.model.as_deref(),
        )),
        _ => return Err(format!("Unknown provider: {}", config.provider)),
    };

    match provider.chat("Respond with just 'OK'", Some("You are a test assistant.")).await {
        Ok(response) => Ok(format!("Connection successful: {}", response.chars().take(50).collect::<String>())),
        Err(e) => Err(format!("Connection failed: {}", e)),
    }
}

#[tauri::command]
fn open_plugin_ui(plugin_id: String, state: State<AudioState>) -> Result<(), String> {
    let chain = state.chain.clone();
    let app_handle = state.app_handle.lock().map_err(|e| e.to_string())?;
    let app_handle = app_handle.clone().ok_or("App handle not set")?;
    
    let mut ui_manager = state.ui_manager.lock().map_err(|e| e.to_string())?;
    
    if ui_manager.is_none() {
        let handle_for_callback = app_handle.clone();
        let callback = Arc::new(move |plugin_id: &str, port_index: u32, value: f32| {
            log::info!("Emitting plugin-parameter-changed: plugin={}, port={}, value={}", plugin_id, port_index, value);
            let _ = handle_for_callback.emit("plugin-parameter-changed", serde_json::json!({
                "pluginId": plugin_id,
                "portIndex": port_index,
                "value": value
            }));
        });
        
        let manager = LV2ExternalUIManager::new(chain, callback)
            .map_err(|e| format!("Failed to create UI manager: {}", e))?;
        *ui_manager = Some(manager);
    }
    
    let manager = ui_manager.as_mut().ok_or("UI manager not initialized")?;
    manager.open_ui(&plugin_id).map_err(|e| e.to_string())
}

#[tauri::command]
fn close_plugin_ui(plugin_id: String, state: State<AudioState>) -> Result<(), String> {
    let mut ui_manager = state.ui_manager.lock().map_err(|e| e.to_string())?;
    
    if let Some(manager) = ui_manager.as_mut() {
        manager.close_ui(&plugin_id).map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
async fn ai_chat(
    message: String,
    audio_state: State<'_, AudioState>,
    app_state: State<'_, AppState>,
) -> Result<CommandList, String> {
    log::debug!("ai_chat called with message: {}", message);
    use std::env;

    let ai_provider: Box<dyn AIProvider> = {
        let config = app_state.config.lock().map_err(|e| e.to_string())?;
        
        if config.is_configured() {
            log::debug!("Using config file provider: {}", config.ai.provider);
            match config.ai.provider.as_str() {
                "siliconflow" => Box::new(OpenAICompatibleProvider::new(
                    AIProviderType::SiliconFlow,
                    &config.ai.api_key,
                    config.ai.model.as_deref(),
                )),
                "deepseek" => Box::new(OpenAICompatibleProvider::new(
                    AIProviderType::DeepSeek,
                    &config.ai.api_key,
                    config.ai.model.as_deref(),
                )),
                "openai" => Box::new(OpenAICompatibleProvider::new(
                    AIProviderType::OpenAI,
                    &config.ai.api_key,
                    config.ai.model.as_deref(),
                )),
                _ => return Err(format!("Unknown provider: {}", config.ai.provider)),
            }
        } else if let Ok(api_key) = env::var("SILICONFLOW_API_KEY") {
            let model = env::var("SILICONFLOW_MODEL").ok();
            log::debug!("Using SiliconFlow provider from env, model: {:?}", model);
            Box::new(OpenAICompatibleProvider::new(
                AIProviderType::SiliconFlow,
                &api_key,
                model.as_deref(),
            ))
        } else if let Ok(api_key) = env::var("DEEPSEEK_API_KEY") {
            let model = env::var("DEEPSEEK_MODEL")
                .or_else(|_| env::var("AI_MODEL"))
                .or_else(|_| env::var("OPENAI_MODEL"))
                .ok();
            log::debug!("Using DeepSeek provider from env, model: {:?}", model);
            Box::new(OpenAICompatibleProvider::new(
                AIProviderType::DeepSeek,
                &api_key,
                model.as_deref(),
            ))
        } else if let Ok(api_key) = env::var("OPENAI_API_KEY") {
            let model = env::var("OPENAI_MODEL").ok();
            log::debug!("Using OpenAI provider from env, model: {:?}", model);
            Box::new(OpenAICompatibleProvider::new(
                AIProviderType::OpenAI,
                &api_key,
                model.as_deref(),
            ))
        } else {
            log::error!("No AI API key configured");
            return Err("No AI API key configured. Configure in Settings or set environment variable.".to_string());
        }
    };

    {
        let mut conversation = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conversation.add_message(MemoryRole::User, &message);
    }

    let response = if ai_provider.supports_tools() {
        log::debug!("Provider supports tools, using process_with_tools");
        process_with_tools(&ai_provider, &message, &audio_state, &app_state).await?
    } else {
        log::debug!("Provider does not support tools, using process_without_tools");
        process_without_tools(&ai_provider, &message, &audio_state, &app_state).await?
    };

    log::debug!("AI response received: {}", response.content.chars().take(200).collect::<String>());

    {
        let mut conversation = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conversation.add_message_with_meta(
            MemoryRole::Assistant,
            &response.content,
            None,
            None,
            None,
            response.reasoning_content.clone(),
        );
        app_state
            .memory_storage
            .save(&conversation)
            .map_err(|e| e.to_string())?;
    }

    let commands = parse_ai_response(&response.content).map_err(|e| {
        log::error!("Failed to parse AI response: {}", e);
        e.to_string()
    })?;
    log::debug!("Returning {} command(s)", commands.commands.len());
    Ok(commands)
}

async fn process_with_tools(
    ai_provider: &Box<dyn AIProvider>,
    _message: &str,
    audio_state: &State<'_, AudioState>,
    app_state: &State<'_, AppState>,
) -> Result<AIResponse, String> {
    log::debug!("process_with_tools: starting");
    const MAX_TOOL_ITERATIONS: usize = 5;

    let tools = get_tool_definitions();
    let history_messages: Vec<ChatMessage> = {
        let conversation = app_state.conversation.lock().map_err(|e| e.to_string())?;
        conversation
            .messages
            .iter()
            .filter(|m| m.role != MemoryRole::System)
            .filter(|m| {
                if m.role == MemoryRole::Assistant 
                    && m.content.is_empty() 
                    && m.tool_calls.clone().is_none_or(|c| c.is_empty())
                    && m.reasoning_content.is_none() 
                {
                    log::warn!("Filtering out invalid empty assistant message without reasoning_content or tool_calls");
                    false
                } else {
                    true
                }
            })
            .map(|m| ChatMessage {
                role: match m.role {
                    MemoryRole::User => MessageRole::User,
                    MemoryRole::Assistant => MessageRole::Assistant,
                    MemoryRole::Tool => MessageRole::Tool,
                    MemoryRole::System => MessageRole::System,
                },
                content: m.content.clone(),
                name: m.name.clone(),
                tool_call_id: m.tool_call_id.clone(),
                tool_calls: m.tool_calls.clone().map(|calls| {
                    calls
                        .iter()
                        .map(|c| ToolCallRequest {
                            id: c.id.clone(),
                            call_type: c.call_type.clone(),
                            function: FunctionCallRequest {
                                name: c.function.name.clone(),
                                arguments: c.function.arguments.clone(),
                            },
                        })
                        .collect()
                }),
                reasoning_content: m.reasoning_content.clone(),
            })
            .collect()
    };

    let mut request = ChatRequest::new()
        .with_system_prompt(SYSTEM_PROMPT)
        .with_tools(tools);

    for msg in history_messages {
        request = request.add_full_message(msg);
    }

    for iteration in 0..MAX_TOOL_ITERATIONS {
        log::debug!("Tool iteration {}", iteration + 1);
        let response = ai_provider
            .chat_with_tools(request.clone())
            .await
            .map_err(|e| e.to_string())?;

        if let Some(ref tool_calls) = response.tool_calls {
            if !tool_calls.is_empty() {
                log::debug!("Received {} tool call(s)", tool_calls.len());
                let tool_calls_for_history: Vec<ToolCall> = tool_calls
                    .iter()
                    .map(|tc| ToolCall {
                        id: tc.id.clone(),
                        call_type: tc.call_type.clone(),
                        function: FunctionCall {
                            name: tc.function.name.clone(),
                            arguments: tc.function.arguments.clone(),
                        },
                    })
                    .collect();

                {
                    let mut conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
                    conv.add_message_with_meta(
                        MemoryRole::Assistant,
                        response.content.as_deref().unwrap_or(""),
                        None,
                        None,
                        Some(tool_calls_for_history),
                        response.reasoning_content.clone(),
                    );
                }

                let tool_results: Vec<(String, String, String)> = {
                    let chain = audio_state.chain.lock().map_err(|e| e.to_string())?;
                    tool_calls
                        .iter()
                        .map(|tool_call| {
                            let result = execute_tool(
                                &tool_call.function.name,
                                &tool_call.function.arguments,
                                &chain,
                            )
                            .unwrap_or_else(|e| format!("Error: {}", e));
                            (
                                tool_call.id.clone(),
                                tool_call.function.name.clone(),
                                result,
                            )
                        })
                        .collect()
                };

                request = request.add_assistant_with_tools_and_reasoning(
                    response.content.as_deref().unwrap_or(""),
                    tool_calls.clone(),
                    response.reasoning_content.clone(),
                );

                for (tool_call_id, tool_name, result) in tool_results {
                    log::debug!("Adding tool result for {}: {} bytes", tool_name, result.len());
                    {
                        let mut conv = app_state.conversation.lock().map_err(|e| e.to_string())?;
                        conv.add_tool_result(&tool_call_id, &tool_name, &result);
                    }
                    request = request.add_tool_result(&tool_call_id, &tool_name, &result);
                }

                log::debug!("Continuing to next iteration with tool results");
                continue;
            }
        }

        if let Some(content) = response.content {
            log::debug!("Got final content response");
            return Ok(AIResponse {
                content,
                reasoning_content: response.reasoning_content.clone(),
            });
        }

        log::warn!("Empty response from AI");
        return Err("Empty response from AI".to_string());
    }

    log::info!("Maximum tool iterations reached, returning summary response");
    Ok(AIResponse {
        content: r#"{"commands": []}"#.to_string(),
        reasoning_content: None,
    })
}

async fn process_without_tools(
    ai_provider: &Box<dyn AIProvider>,
    message: &str,
    audio_state: &State<'_, AudioState>,
    app_state: &State<'_, AppState>,
) -> Result<AIResponse, String> {
    let (plugins_info, active_plugins, bypass) = {
        let chain = audio_state.chain.lock().map_err(|e| e.to_string())?;
        let plugins_info: Vec<String> = chain
            .list_available_plugins()
            .iter()
            .take(30)
            .filter_map(|p| {
                let params = chain.get_plugin_parameters(&p.uri)?;
                let params_str: Vec<String> = params
                    .iter()
                    .take(10)
                    .map(|x| format!("{}:{:.0}-{:.0}", x.name, x.min, x.max))
                    .collect();
                Some(format!(
                    "- {} ({}) [{}]\n  params: {}",
                    p.name,
                    p.plugin_type,
                    p.uri,
                    params_str.join(", ")
                ))
            })
            .collect();

        let active_plugins: Vec<String> = chain
            .get_active_plugins()
            .iter()
            .map(|p| {
                let params_str: Vec<String> = p
                    .parameters
                    .iter()
                    .take(5)
                    .map(|x| x.name.clone())
                    .collect();
                format!(
                    "- {} (ID: {}) params: {}",
                    p.info.name,
                    p.id,
                    params_str.join(", ")
                )
            })
            .collect();

        (plugins_info, active_plugins, chain.bypass())
    };

    let history_context: Vec<String> = {
        let conversation = app_state.conversation.lock().map_err(|e| e.to_string())?;
        let mut history_vec: Vec<_> = conversation
            .messages
            .iter()
            .filter(|m| matches!(m.role, MemoryRole::User | MemoryRole::Assistant))
            .collect();
        history_vec.reverse();
        history_vec
            .iter()
            .take(6)
            .rev()
            .map(|m| {
                format!(
                    "{}: {}",
                    match m.role {
                        MemoryRole::User => "User",
                        MemoryRole::Assistant => "Assistant",
                        _ => "",
                    },
                    m.content.chars().take(200).collect::<String>()
                )
            })
            .collect()
    };

    let prompt = format!(
        "Recent conversation:\n{}\n\nUser request: {}\n\nAvailable plugins with parameters:\n{}\n\nActive plugins:\n{}\n\nBypass: {}",
        history_context.join("\n"),
        message,
        plugins_info.join("\n"),
        if active_plugins.is_empty() {
            "None".to_string()
        } else {
            active_plugins.join("\n")
        },
        if bypass { "ON" } else { "OFF" }
    );

    let response = ai_provider
        .chat(&prompt, Some(SYSTEM_PROMPT_WITH_CONTEXT))
        .await
        .map_err(|e| e.to_string())?;

    Ok(AIResponse {
        content: response,
        reasoning_content: None,
    })
}

pub fn run() {
    let chain = Arc::new(Mutex::new(PluginChain::new().unwrap()));

    let audio_state = Arc::new(Mutex::new(JackAudioState {
        process_fn: Box::new(|_input: &[f32], _output: &mut [f32], _nframes: usize| {}),
    }));

    let jack_engine = JackEngine::new("estima", audio_state.clone())
        .expect("Failed to create JACK client. Is JACK running?");

    log::info!("JACK client '{}' created", jack_engine.client_name());
    let sample_rate = jack_engine.sample_rate() as f64;
    log::info!(
        "Sample rate: {}, Buffer size: {}",
        sample_rate,
        jack_engine.buffer_size()
    );

    {
        let chain_clone = chain.clone();
        let mut state = audio_state.lock().unwrap();
        state.process_fn = Box::new(move |input: &[f32], output: &mut [f32], nframes: usize| {
            if let Ok(mut c) = chain_clone.lock() {
                c.process(input, output, nframes);
            }
        });
    }

    let _ = jack_engine;

    let memory_storage = MemoryStorage::new().expect("Failed to initialize memory storage");
    let conversation = match memory_storage.load() {
        Ok(conv) => Arc::new(Mutex::new(conv)),
        Err(_) => Arc::new(Mutex::new(Conversation::new())),
    };

    let config = Arc::new(Mutex::new(
        AppConfig::load().unwrap_or_default()
    ));

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AudioState {
            chain,
            last_loaded_id: Arc::new(Mutex::new(None)),
            sample_rate,
            started: true,
            ui_manager: Arc::new(Mutex::new(None)),
            app_handle: Arc::new(Mutex::new(None)),
        })
        .setup(|app| {
            // Get the app handle and store it in AudioState
            let handle = app.handle().clone();
            let state = app.state::<AudioState>();
            *state.app_handle.lock().unwrap() = Some(handle);
            Ok(())
        })
        .manage(AppState {
            memory_storage,
            conversation,
            config,
        })
        .invoke_handler(tauri::generate_handler![
            list_plugins,
            load_plugin,
            remove_plugin,
            toggle_plugin_bypass,
            move_plugin,
            get_plugin_parameters,
            get_active_plugin_parameters,
            set_parameter,
            get_chain_status,
            toggle_bypass,
            save_preset,
            load_preset,
            list_presets,
            ai_chat,
            get_history,
            clear_history,
            get_config,
            save_config,
            test_ai_connection,
            open_plugin_ui,
            close_plugin_ui,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
