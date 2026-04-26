use anyhow::{anyhow, Result};
use std::sync::{Arc, Mutex};
use dialoguer::Input;
use estima_core::audio::{JackEngine, PluginChain, AudioState};
use estima_core::ai::{AIProvider, OllamaProvider, OpenAICompatibleProvider, AIProviderType};
use estima_core::control::{Command, CommandList};

#[derive(Debug, Clone)]
enum AIConfig {
    Ollama { model: Option<String> },
    OpenAICompatible { 
        provider_type: AIProviderType,
        api_key: String, 
        model: Option<String> 
    },
}

impl AIConfig {
    fn create_provider(&self) -> Box<dyn AIProvider> {
        match self {
            AIConfig::Ollama { model } => {
                Box::new(OllamaProvider::new(None, model.as_deref()))
            }
            AIConfig::OpenAICompatible { provider_type, api_key, model } => {
                Box::new(OpenAICompatibleProvider::new(provider_type.clone(), api_key, model.as_deref()))
            }
        }
    }
    
    fn display_name(&self) -> String {
        match self {
            AIConfig::Ollama { model } => {
                format!("Ollama (local, model: {})", model.as_deref().unwrap_or("llama3"))
            }
            AIConfig::OpenAICompatible { provider_type, model, .. } => {
                format!("{} ({})", provider_type.display_name(), model.as_deref().unwrap_or_else(|| provider_type.default_model()))
            }
        }
    }
}

fn detect_ai_config() -> Option<AIConfig> {
    if let Ok(api_key) = std::env::var("SILICONFLOW_API_KEY") {
        let model = std::env::var("SILICONFLOW_MODEL")
            .or_else(|_| std::env::var("AI_MODEL"))
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .ok();
        return Some(AIConfig::OpenAICompatible {
            provider_type: AIProviderType::SiliconFlow,
            api_key,
            model,
        });
    }
    
    if let Ok(api_key) = std::env::var("DEEPSEEK_API_KEY") {
        let model = std::env::var("DEEPSEEK_MODEL")
            .or_else(|_| std::env::var("AI_MODEL"))
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .ok();
        return Some(AIConfig::OpenAICompatible {
            provider_type: AIProviderType::DeepSeek,
            api_key,
            model,
        });
    }
    
    if let Ok(api_key) = std::env::var("ANTHROPIC_API_KEY") {
        let model = std::env::var("ANTHROPIC_MODEL")
            .or_else(|_| std::env::var("AI_MODEL"))
            .or_else(|_| std::env::var("OPENAI_MODEL"))
            .ok();
        return Some(AIConfig::OpenAICompatible {
            provider_type: AIProviderType::Anthropic,
            api_key,
            model,
        });
    }
    
    if let Ok(api_key) = std::env::var("OPENAI_API_KEY") {
        let model = std::env::var("OPENAI_MODEL")
            .or_else(|_| std::env::var("AI_MODEL"))
            .ok();
        return Some(AIConfig::OpenAICompatible {
            provider_type: AIProviderType::OpenAI,
            api_key,
            model,
        });
    }
    
    if let Ok(api_key) = std::env::var("AI_API_KEY") {
        let base_url = std::env::var("AI_BASE_URL")
            .unwrap_or_else(|_| "https://api.openai.com/v1".to_string());
        let model = std::env::var("AI_MODEL").ok();
        return Some(AIConfig::OpenAICompatible {
            provider_type: AIProviderType::Custom(base_url),
            api_key,
            model,
        });
    }
    
    let model = std::env::var("OLLAMA_MODEL").ok();
    Some(AIConfig::Ollama { model })
}

struct App {
    plugin_chain: Arc<Mutex<PluginChain>>,
    ai: Option<Box<dyn AIProvider>>,
    sample_rate: usize,
    rt: tokio::runtime::Runtime,
    last_loaded_id: Option<String>,
}

impl App {
    fn new(ai_config: Option<AIConfig>) -> Result<Self> {
        let plugin_chain = Arc::new(Mutex::new(PluginChain::new()?));
        
        let audio_state = Arc::new(Mutex::new(AudioState {
            process_fn: Box::new(|_input: &[f32], _output: &mut [f32], _nframes: usize| {}),
        }));
        
        let jack_engine = JackEngine::new("estima", audio_state.clone())?;
        
        log::info!("JACK client '{}' created", jack_engine.client_name());
        let sample_rate = jack_engine.sample_rate();
        log::info!("Sample rate: {}, Buffer size: {}", sample_rate, jack_engine.buffer_size());
        
        {
            let chain = plugin_chain.clone();
            let mut state = audio_state.lock().unwrap();
            state.process_fn = Box::new(move |input: &[f32], output: &mut [f32], nframes: usize| {
                if let Ok(mut chain) = chain.lock() {
                    chain.process(input, output, nframes);
                }
            });
        }
        
        let ai = ai_config.as_ref().map(|c| c.create_provider());
        let rt = tokio::runtime::Runtime::new()?;
        
        Ok(Self {
            plugin_chain,
            ai,
            sample_rate,
            rt,
            last_loaded_id: None,
        })
    }

    fn process_command(&mut self, input: &str) -> Result<()> {
        let input = input.trim();
        
        if input.starts_with('/') {
            self.handle_slash_command(input)?;
        } else if self.ai.is_some() {
            let ai = self.ai.take().unwrap();
            let result = self.handle_ai_command(input, ai.as_ref());
            self.ai = Some(ai);
            result?;
        } else {
            println!("No AI provider configured. Use / commands.");
        }
        
        Ok(())
    }

    fn handle_slash_command(&mut self, input: &str) -> Result<()> {
        let parts: Vec<&str> = input.split_whitespace().collect();
        let cmd = parts[0];
        let args = &parts[1..];

        match cmd {
            "/list" | "/l" => {
                let filter = args.first().map(|s| s.to_string());
                let chain = self.plugin_chain.lock().unwrap();
                let plugins = if let Some(ref f) = filter {
                    chain.search_plugins(f)
                } else {
                    chain.list_available_plugins().iter().collect()
                };
                
                if plugins.is_empty() {
                    println!("No plugins found.");
                } else {
                    println!("Found {} plugins:", plugins.len());
                    for (i, p) in plugins.iter().enumerate() {
                        println!("{}. {} [{}]", i + 1, p.name, p.plugin_type);
                        println!("   URI: {}", p.uri);
                    }
                }
            }
            "/params" => {
                if args.is_empty() {
                    let chain = self.plugin_chain.lock().unwrap();
                    if let Some(ref id) = self.last_loaded_id {
                        if let Some(plugin) = chain.get_plugin_by_id(id) {
                            println!("Parameters for {}:", plugin.info.name);
                            for p in &plugin.parameters {
                                println!("  - {} (range: {:.2} - {:.2}, default: {:.2})", 
                                    p.name, p.min, p.max, p.default);
                            }
                        }
                    } else {
                        println!("No plugin loaded. Use /load first or specify plugin URI.");
                    }
                } else {
                    let uri = args.join(" ");
                    let chain = self.plugin_chain.lock().unwrap();
                    if let Some(params) = chain.get_plugin_parameters(&uri) {
                        println!("Parameters:");
                        for p in &params {
                            println!("  - {} (range: {:.2} - {:.2}, default: {:.2})", 
                                p.name, p.min, p.max, p.default);
                        }
                    } else {
                        println!("Plugin not found: {}", uri);
                    }
                }
            }
            "/status" | "/s" => {
                let chain = self.plugin_chain.lock().unwrap();
                let bypass_status = if chain.bypass() { " [BYPASS]" } else { "" };
                let plugins = chain.get_active_plugins();
                if plugins.is_empty() {
                    println!("No active plugins.{}", bypass_status);
                } else {
                    println!("Active plugins ({}):{}", plugins.len(), bypass_status);
                    for (i, p) in plugins.iter().enumerate() {
                        println!("{}. {} [{}]", i + 1, p.info.name, p.info.plugin_type);
                    }
                }
            }
            "/bypass" | "/b" => {
                let mut chain = self.plugin_chain.lock().unwrap();
                let new_state = chain.toggle_bypass();
                if new_state {
                    println!("BYPASS ON - Audio passes through without effects");
                } else {
                    println!("BYPASS OFF - Effects processing active");
                }
            }
            "/clear" | "/c" => {
                let mut chain = self.plugin_chain.lock().unwrap();
                chain.clear();
                self.last_loaded_id = None;
                println!("Cleared all plugins.");
            }
            "/load" => {
                if args.is_empty() {
                    return Err(anyhow!("Usage: /load <plugin_uri>"));
                }
                let uri = args.join(" ");
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.load_plugin(&uri, self.sample_rate as f64) {
                    Ok((id, params)) => {
                        self.last_loaded_id = Some(id.clone());
                        let plugin = chain.get_active_plugins().iter().find(|p| p.id == id);
                        let name = plugin.map(|p| p.info.name.as_str()).unwrap_or(&uri);
                        println!("Loaded: {} (ID: {})", name, id);
                        println!("Parameters:");
                        for p in &params {
                            println!("  - {}", p);
                        }
                    }
                    Err(e) => println!("Failed to load: {}", e),
                }
            }
            "/remove" | "/rm" => {
                if args.is_empty() {
                    return Err(anyhow!("Usage: /remove <plugin_id>"));
                }
                let id = args[0];
                let target_id = if id == "@last" || id == "@l" {
                    self.last_loaded_id.clone().unwrap_or_default()
                } else {
                    id.to_string()
                };
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.remove_plugin(&target_id) {
                    Ok(()) => {
                        println!("Removed plugin: {}", target_id);
                        if self.last_loaded_id.as_ref() == Some(&target_id) {
                            self.last_loaded_id = None;
                        }
                    }
                    Err(e) => println!("Failed to remove: {}", e),
                }
            }
            "/param" | "/p" => {
                if args.len() < 3 {
                    return Err(anyhow!("Usage: /param <plugin_id|@last> <param_name> <value>"));
                }
                let target_id = if args[0] == "@last" || args[0] == "@l" {
                    self.last_loaded_id.clone().unwrap_or_default()
                } else {
                    args[0].to_string()
                };
                let param_name = args[1];
                let value: f32 = args[2].parse().map_err(|_| anyhow!("Invalid value"))?;
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.set_parameter(&target_id, param_name, value) {
                    Ok(()) => println!("Set {} = {}", param_name, value),
                    Err(e) => println!("Failed: {}", e),
                }
            }
            "/save" => {
                let name = args.first().map(|s| s.to_string()).unwrap_or_else(|| "preset".to_string());
                let chain = self.plugin_chain.lock().unwrap();
                match chain.save_config(&name, None) {
                    Ok(path) => println!("Saved preset to: {}", path.display()),
                    Err(e) => println!("Failed to save: {}", e),
                }
            }
            "/open" => {
                if args.is_empty() {
                    let presets = PluginChain::list_presets()?;
                    if presets.is_empty() {
                        println!("No presets found. Use /save <name> to create one.");
                    } else {
                        println!("Available presets:");
                        for (i, (name, _)) in presets.iter().enumerate() {
                            println!("  {}. {}", i + 1, name);
                        }
                        println!("Use /open <name> to load a preset.");
                    }
                } else {
                    let name = args[0];
                    let path = format!("{}.estima.json", name);
                    let mut chain = self.plugin_chain.lock().unwrap();
                    match chain.load_config(&path, self.sample_rate as f64) {
                        Ok(config) => {
                            self.last_loaded_id = chain.get_active_plugins().last().map(|p| p.id.clone());
                            println!("Loaded preset: {} ({} plugins)", config.name, config.plugins.len());
                            if config.bypass {
                                println!("Bypass is ON");
                            }
                        }
                        Err(e) => println!("Failed to load: {}", e),
                    }
                }
            }
            "/presets" => {
                let presets = PluginChain::list_presets()?;
                if presets.is_empty() {
                    println!("No presets found. Use /save <name> to create one.");
                } else {
                    println!("Available presets:");
                    for (i, (name, path)) in presets.iter().enumerate() {
                        println!("  {}. {} ({})", i + 1, name, path.display());
                    }
                }
            }
            "/help" | "/h" | "/?" => {
                self.print_help();
            }
            "/quit" | "/q" | "/exit" => {
                println!("Goodbye!");
                std::process::exit(0);
            }
            _ => {
                return Err(anyhow!("Unknown command: {}. Type /help for help.", cmd));
            }
        }

        Ok(())
    }

    fn handle_ai_command(&mut self, input: &str, ai: &dyn AIProvider) -> Result<()> {
        println!("Thinking...");

        let command_list: CommandList = self.rt.block_on(async {
            Self::interpret_natural_language(ai, input, &self.plugin_chain).await
        })?;
        
        println!("AI commands ({}):", command_list.commands.len());
        for (i, cmd) in command_list.commands.iter().enumerate() {
            println!("  {}. {:?}", i + 1, cmd);
        }

        for command in command_list.commands {
            self.execute_command(command)?;
        }

        Ok(())
    }
    
    fn execute_command(&mut self, command: Command) -> Result<()> {
        match command {
            Command::LoadPlugin { uri, .. } => {
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.load_plugin(&uri, self.sample_rate as f64) {
                    Ok((id, params)) => {
                        self.last_loaded_id = Some(id.clone());
                        let plugin = chain.get_active_plugins().iter().find(|p| p.id == id);
                        let name = plugin.map(|p| p.info.name.as_str()).unwrap_or(&uri);
                        println!("✓ Loaded: {}", name);
                        if !params.is_empty() {
                            print!("  Params: ");
                            let names: Vec<&str> = params.iter().take(5).map(|x| x.name.as_str()).collect();
                            println!("{}", names.join(", "));
                        }
                    }
                    Err(e) => println!("✗ Failed: {}", e),
                }
            }
            Command::RemovePlugin { id } => {
                let target_id = if id == "@last" { 
                    self.last_loaded_id.clone().unwrap_or_default() 
                } else { 
                    id 
                };
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.remove_plugin(&target_id) {
                    Ok(()) => {
                        println!("✓ Removed plugin");
                        if self.last_loaded_id.as_ref() == Some(&target_id) {
                            self.last_loaded_id = None;
                        }
                    }
                    Err(e) => println!("✗ Failed: {}", e),
                }
            }
            Command::ClearChain { .. } => {
                let mut chain = self.plugin_chain.lock().unwrap();
                chain.clear();
                self.last_loaded_id = None;
                println!("✓ Cleared all plugins");
            }
            Command::SetParameter { plugin_id, param_name, value } => {
                let target_id = if plugin_id == "@last" { 
                    self.last_loaded_id.clone().unwrap_or_default() 
                } else { 
                    plugin_id 
                };
                let mut chain = self.plugin_chain.lock().unwrap();
                match chain.set_parameter(&target_id, &param_name, value) {
                    Ok(()) => println!("✓ Set {} = {}", param_name, value),
                    Err(e) => println!("✗ Failed: {}", e),
                }
            }
            Command::SetBypass { bypass } => {
                let mut chain = self.plugin_chain.lock().unwrap();
                chain.set_bypass(bypass);
                if bypass {
                    println!("✓ Bypass ON");
                } else {
                    println!("✓ Bypass OFF");
                }
            }
            Command::ShowStatus { .. } => {
                let chain = self.plugin_chain.lock().unwrap();
                let bypass_status = if chain.bypass() { " [BYPASS]" } else { "" };
                let plugins = chain.get_active_plugins();
                if plugins.is_empty() {
                    println!("No active plugins.{}", bypass_status);
                } else {
                    println!("Active plugins ({}):{}", plugins.len(), bypass_status);
                    for (i, p) in plugins.iter().enumerate() {
                        println!("  {}. {} [{}]", i + 1, p.info.name, p.info.plugin_type);
                    }
                }
            }
            Command::ListPlugins { filter } => {
                let chain = self.plugin_chain.lock().unwrap();
                let plugins = if let Some(ref f) = filter {
                    chain.search_plugins_with_params(f)
                } else {
                    chain.list_available_plugins().iter()
                        .filter_map(|p| {
                            chain.get_plugin_parameters(&p.uri)
                                .map(|params| (p, params))
                        })
                        .collect()
                };
                println!("✓ Found {} plugins", plugins.len());
                for (i, (p, params)) in plugins.iter().take(3).enumerate() {
                    println!("  {}. {} [{}]", i + 1, p.name, p.plugin_type);
                    if !params.is_empty() {
                        print!("     Params: ");
                        let names: Vec<&str> = params.iter().take(5).map(|x| x.name.as_str()).collect();
                        println!("{}", names.join(", "));
                    }
                }
                if plugins.len() > 3 {
                    println!("  ... and {} more", plugins.len() - 3);
                }
            }
            _ => {
                println!("✗ Command not implemented");
            }
        }
        
        Ok(())
    }

    async fn interpret_natural_language(
        ai: &dyn AIProvider,
        input: &str,
        plugin_chain: &Arc<Mutex<PluginChain>>,
    ) -> Result<CommandList> {
        let (plugins_info, active_plugins, bypass) = {
            let chain = plugin_chain.lock().unwrap();
            
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

        let message = format!(
            "User request: {}\n\nAvailable plugins with parameters:\n{}\n\nActive plugins:\n{}\n\nBypass: {}",
            input,
            plugins_info.join("\n"),
            if active_plugins.is_empty() { "None".to_string() } else { active_plugins.join("\n") },
            if bypass { "ON" } else { "OFF" }
        );

        let response = ai.chat(&message, Some(SYSTEM_PROMPT)).await?;
        
        let parsed: CommandList = parse_ai_response(&response)?;
        Ok(parsed)
    }

    fn print_help(&self) {
        println!("Estima - AI-Controlled Real-Time Effects Processor");
        println!();
        println!("Plugin Commands:");
        println!("  /list [filter]     List available plugins");
        println!("  /load <uri>        Load a plugin by URI");
        println!("  /remove <id|@last> Remove a plugin");
        println!("  /param <id|@last> <name> <value>  Set parameter");
        println!("  /params [uri]      Show parameters for plugin");
        println!("  /status            Show current plugin chain");
        println!("  /clear             Clear all plugins");
        println!();
        println!("Bypass:");
        println!("  /bypass            Toggle bypass on/off");
        println!();
        println!("Presets:");
        println!("  /save [name]       Save current config as preset");
        println!("  /open [name]       Load a preset (list if no name)");
        println!("  /presets           List available presets");
        println!();
        println!("Other:");
        println!("  /help              Show this help");
        println!("  /quit              Exit");
        println!();
        println!("Use @last to reference the most recently loaded plugin.");
        println!();
        println!("Natural Language:");
        println!("  \"Add a reverb effect\"");
        println!("  \"Give me more distortion\"");
        println!("  \"Bypass the effects\"");
        println!("  \"Remove all effects\"");
    }

    fn run(&mut self) -> Result<()> {
        println!("Estima - AI-Controlled Real-Time Effects Processor");
        println!("Type /help for commands, or describe what you want.");
        println!();

        loop {
            let prompt = {
                let chain = self.plugin_chain.lock().unwrap();
                let bypass = if chain.bypass() { "[BYP] " } else { "" };
                let count = chain.get_active_plugins().len();
                format!("{}estima({})", bypass, count)
            };
            
            let input: String = Input::new()
                .with_prompt(prompt.as_str())
                .interact_text()
                .map_err(|e| anyhow!("Input error: {}", e))?;

            if input.trim().is_empty() {
                continue;
            }

            if let Err(e) = self.process_command(&input) {
                println!("Error: {}", e);
            }
        }
    }
}

use estima_core::control::{parse_ai_response, SYSTEM_PROMPT};

fn main() -> Result<()> {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    let ai_config = detect_ai_config();
    if let Some(ref config) = ai_config {
        println!("Using AI provider: {}", config.display_name());
    } else {
        println!("No AI provider configured. Use / commands only.");
        println!("Set: SILICONFLOW_API_KEY, DEEPSEEK_API_KEY, OPENAI_API_KEY, or OLLAMA_MODEL");
    }

    let mut app = App::new(ai_config)?;
    app.run()
}
