use tauri::State;
use std::sync::{Arc, Mutex};
use estima_core::audio::{AudioState as JackAudioState, JackEngine, PluginChain};
use estima_core::ai::{AIProvider, OpenAICompatibleProvider, AIProviderType};
use estima_core::control::{parse_ai_response, CommandList};
use serde::{Deserialize, Serialize};
use anyhow::Result;

#[derive(Serialize, Deserialize, Clone)]
pub struct PluginInfo {
    pub id: String,
    pub uri: String,
    pub name: String,
    pub plugin_type: String,
    pub bypass: bool,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ParameterInfo {
    pub name: String,
    pub symbol: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
    pub current: f32,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct ChainStatus {
    pub plugins: Vec<PluginInfo>,
    pub bypass: bool,
    pub last_loaded_id: Option<String>,
}

pub struct AudioState {
    pub chain: Arc<Mutex<PluginChain>>,
    pub last_loaded_id: Arc<Mutex<Option<String>>>,
    pub sample_rate: f64,
    pub started: bool,
}

#[tauri::command]
fn list_plugins(filter: Option<String>, state: State<AudioState>) -> Result<Vec<PluginInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let plugins: Vec<PluginInfo> = if let Some(f) = filter {
        chain.search_plugins(&f)
            .into_iter()
            .map(|p| PluginInfo {
                id: p.uri.clone(),
                uri: p.uri.clone(),
                name: p.name.clone(),
                plugin_type: p.plugin_type.clone(),
                bypass: false,
            })
            .collect()
    } else {
        chain.list_available_plugins()
            .iter()
            .map(|p| PluginInfo {
                id: p.uri.clone(),
                uri: p.uri.clone(),
                name: p.name.clone(),
                plugin_type: p.plugin_type.clone(),
                bypass: false,
            })
            .collect()
    };
    Ok(plugins)
}

#[tauri::command]
fn load_plugin(uri: String, state: State<AudioState>) -> Result<PluginInfo, String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    let (id, _params) = chain.load_plugin(&uri, state.sample_rate).map_err(|e| e.to_string())?;
    
    let mut last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    *last_id = Some(id.clone());
    
    let plugin = chain.get_active_plugins()
        .iter()
        .find(|p| p.id == id)
        .ok_or("Plugin not found after loading")?;
    
    Ok(PluginInfo {
        id,
        uri,
        name: plugin.info.name.clone(),
        plugin_type: plugin.info.plugin_type.clone(),
        bypass: plugin.bypass,
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
fn get_plugin_parameters(uri: String, state: State<AudioState>) -> Result<Vec<ParameterInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let params = chain.get_plugin_parameters(&uri)
        .ok_or("Plugin not found")?;
    
    Ok(params.into_iter().map(|p| ParameterInfo {
        name: p.name,
        symbol: p.symbol,
        default: p.default,
        min: p.min,
        max: p.max,
        current: p.default,
    }).collect())
}

#[tauri::command]
fn get_active_plugin_parameters(plugin_id: String, state: State<AudioState>) -> Result<Vec<ParameterInfo>, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let plugin = chain.get_plugin_by_id(&plugin_id)
        .ok_or("Plugin not found")?;
    
    Ok(plugin.parameters.iter().zip(plugin.parameter_values.iter())
        .map(|(p, (_, value))| ParameterInfo {
            name: p.name.clone(),
            symbol: p.symbol.clone(),
            default: p.default,
            min: p.min,
            max: p.max,
            current: *value,
        })
        .collect())
}

#[tauri::command]
fn set_parameter(plugin_id: String, param_name: String, value: f32, state: State<AudioState>) -> Result<(), String> {
    let mut chain = state.chain.lock().map_err(|e| e.to_string())?;
    
    let target_id = if plugin_id == "@last" {
        state.last_loaded_id.lock().map_err(|e| e.to_string())?
            .clone()
            .ok_or("No plugin has been loaded yet")?
    } else {
        plugin_id
    };
    
    chain.set_parameter(&target_id, &param_name, value).map_err(|e| e.to_string())?;
    Ok(())
}

#[tauri::command]
fn get_chain_status(state: State<AudioState>) -> Result<ChainStatus, String> {
    let chain = state.chain.lock().map_err(|e| e.to_string())?;
    let last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    
    let plugins: Vec<PluginInfo> = chain.get_active_plugins()
        .iter()
        .map(|p| PluginInfo {
            id: p.id.clone(),
            uri: p.info.uri.clone(),
            name: p.info.name.clone(),
            plugin_type: p.info.plugin_type.clone(),
            bypass: p.bypass,
        })
        .collect();
    
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
    let _config = chain.load_config(&path, state.sample_rate).map_err(|e| e.to_string())?;
    
    let mut last_id = state.last_loaded_id.lock().map_err(|e| e.to_string())?;
    if let Some(last) = chain.get_active_plugins().last() {
        *last_id = Some(last.id.clone());
    }
    
    let plugins: Vec<PluginInfo> = chain.get_active_plugins()
        .iter()
        .map(|p| PluginInfo {
            id: p.id.clone(),
            uri: p.info.uri.clone(),
            name: p.info.name.clone(),
            plugin_type: p.info.plugin_type.clone(),
            bypass: p.bypass,
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
async fn ai_chat(message: String, state: State<'_, AudioState>) -> Result<CommandList, String> {
    use std::env;
    
    let ai_provider: Box<dyn AIProvider> = if let Ok(api_key) = env::var("SILICONFLOW_API_KEY") {
        let model = env::var("SILICONFLOW_MODEL").ok();
        Box::new(OpenAICompatibleProvider::new(
            AIProviderType::SiliconFlow,
            &api_key,
            model.as_deref()
        ))
    } else if let Ok(api_key) = env::var("OPENAI_API_KEY") {
        let model = env::var("OPENAI_MODEL").ok();
        Box::new(OpenAICompatibleProvider::new(
            AIProviderType::OpenAI,
            &api_key,
            model.as_deref()
        ))
    } else {
        return Err("No AI API key configured. Set SILICONFLOW_API_KEY or OPENAI_API_KEY".to_string());
    };
    
    let (plugins_info, active_plugins, bypass) = {
        let chain = state.chain.lock().map_err(|e| e.to_string())?;
        let plugins_info: Vec<String> = chain
            .list_available_plugins()
            .iter()
            .take(20)
            .filter_map(|p| {
                let params = chain.get_plugin_parameters(&p.uri)?;
                let params_str: Vec<String> = params.iter()
                    .take(8)
                    .map(|x| format!("{}:{:.0}-{:.0}", x.name, x.min, x.max))
                    .collect();
                Some(format!("- {} ({}) [{}]\n  params: {}",
                    p.name, p.plugin_type, p.uri, params_str.join(", ")))
            })
            .collect();

        let active_plugins: Vec<String> = chain
            .get_active_plugins()
            .iter()
            .map(|p| {
                let params_str: Vec<String> = p.parameters.iter()
                    .take(5)
                    .map(|x| x.name.clone())
                    .collect();
                format!("- {} (ID: {}) params: {}", p.info.name, p.id, params_str.join(", "))
            })
            .collect();
        
        (plugins_info, active_plugins, chain.bypass())
    };

    let prompt = format!(
        "User request: {}\n\nAvailable plugins with parameters:\n{}\n\nActive plugins:\n{}\n\nBypass: {}",
        message,
        plugins_info.join("\n"),
        if active_plugins.is_empty() { "None".to_string() } else { active_plugins.join("\n") },
        if bypass { "ON" } else { "OFF" }
    );

    let system_prompt = estima_core::control::SYSTEM_PROMPT;
    let response = ai_provider.chat(&prompt, Some(system_prompt)).await
        .map_err(|e| e.to_string())?;
    
    let commands = parse_ai_response(&response).map_err(|e| e.to_string())?;
    Ok(commands)
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
    log::info!("Sample rate: {}, Buffer size: {}", sample_rate, jack_engine.buffer_size());
    
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

    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .manage(AudioState {
            chain,
            last_loaded_id: Arc::new(Mutex::new(None)),
            sample_rate,
            started: true,
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
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}