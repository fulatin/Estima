use anyhow::{anyhow, Result};
use livi::{event::LV2AtomSequence, FeaturesBuilder, Instance, PortConnections, PortType, World};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone)]
pub struct PluginInfo {
    pub uri: String,
    pub name: String,
    pub plugin_type: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ParameterInfo {
    pub name: String,
    pub symbol: String,
    pub default: f32,
    pub min: f32,
    pub max: f32,
}

impl std::fmt::Display for ParameterInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} ({:.2} - {:.2}, default: {:.2})",
            self.name, self.min, self.max, self.default
        )
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginConfig {
    pub uri: String,
    pub name: String,
    pub parameters: Vec<(String, f32)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChainConfig {
    pub version: String,
    pub name: String,
    pub bypass: bool,
    pub plugins: Vec<PluginConfig>,
}

pub struct ActivePlugin {
    pub id: String,
    pub instance: Instance,
    pub info: PluginInfo,
    pub features: Arc<livi::Features>,
    pub parameters: Vec<ParameterInfo>,
    pub parameter_values: Vec<(String, f32)>,
    atom_sequence_inputs: Vec<LV2AtomSequence>,
    atom_sequence_outputs: Vec<LV2AtomSequence>,
}

pub struct PluginChain {
    world: World,
    plugins: Vec<ActivePlugin>,
    available_plugins: Vec<PluginInfo>,
    features: Arc<livi::Features>,
    bypass: bool,
}

unsafe impl Send for PluginChain {}
unsafe impl Sync for PluginChain {}

impl PluginChain {
    pub fn new() -> Result<Self> {
        let world = World::new();
        let features = world.build_features(FeaturesBuilder::default());
        let available_plugins = Self::enumerate_plugins(&world);

        Ok(Self {
            world,
            plugins: Vec::new(),
            available_plugins,
            features,
            bypass: false,
        })
    }

    pub fn bypass(&self) -> bool {
        self.bypass
    }

    pub fn set_bypass(&mut self, bypass: bool) {
        self.bypass = bypass;
    }

    pub fn toggle_bypass(&mut self) -> bool {
        self.bypass = !self.bypass;
        self.bypass
    }

    fn enumerate_plugins(world: &World) -> Vec<PluginInfo> {
        let mut plugins: Vec<PluginInfo> = world
            .iter_plugins()
            .map(|plugin| {
                let name = plugin.name();
                let uri = plugin.uri();
                let plugin_type = Self::classify_plugin(&name);

                PluginInfo {
                    uri,
                    name,
                    plugin_type,
                }
            })
            .collect();

        plugins.sort_by(|a, b| a.name.cmp(&b.name));
        plugins
    }

    fn classify_plugin(name: &str) -> String {
        let name_lower = name.to_lowercase();

        if name_lower.contains("reverb")
            || name_lower.contains("hall")
            || name_lower.contains("room")
        {
            "reverb".to_string()
        } else if name_lower.contains("delay") || name_lower.contains("echo") {
            "delay".to_string()
        } else if name_lower.contains("distort")
            || name_lower.contains("overdrive")
            || name_lower.contains("fuzz")
            || name_lower.contains("crusher")
        {
            "distortion".to_string()
        } else if name_lower.contains("compress") || name_lower.contains("limiter") {
            "dynamics".to_string()
        } else if name_lower.contains("eq")
            || name_lower.contains("equalizer")
            || name_lower.contains("filter")
        {
            "eq".to_string()
        } else if name_lower.contains("chorus")
            || name_lower.contains("flanger")
            || name_lower.contains("phaser")
        {
            "modulation".to_string()
        } else if name_lower.contains("amp")
            || name_lower.contains("cabinet")
            || name_lower.contains("cab")
        {
            "amp".to_string()
        } else if name_lower.contains("gain") || name_lower.contains("volume") {
            "gain".to_string()
        } else if name_lower.contains("noise") || name_lower.contains("gate") {
            "noise".to_string()
        } else {
            "other".to_string()
        }
    }

    fn extract_parameters(plugin: &livi::Plugin) -> Vec<ParameterInfo> {
        plugin
            .ports()
            .filter(|p| matches!(p.port_type, PortType::ControlInput))
            .map(|p| ParameterInfo {
                name: p.name.clone(),
                symbol: p.symbol.clone(),
                default: p.default_value,
                min: p.min_value.unwrap_or(0.0),
                max: p.max_value.unwrap_or(1.0),
            })
            .collect()
    }

    pub fn get_plugin_parameters(&self, uri: &str) -> Option<Vec<ParameterInfo>> {
        self.world
            .plugin_by_uri(uri)
            .map(|p| Self::extract_parameters(&p))
    }

    pub fn list_available_plugins(&self) -> &[PluginInfo] {
        &self.available_plugins
    }

    pub fn search_plugins(&self, query: &str) -> Vec<&PluginInfo> {
        let query_lower = query.to_lowercase();
        self.available_plugins
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.plugin_type.to_lowercase().contains(&query_lower)
                    || p.uri.to_lowercase().contains(&query_lower)
            })
            .collect()
    }

    pub fn search_plugins_with_params(
        &self,
        query: &str,
    ) -> Vec<(&PluginInfo, Vec<ParameterInfo>)> {
        let query_lower = query.to_lowercase();
        self.available_plugins
            .iter()
            .filter(|p| {
                p.name.to_lowercase().contains(&query_lower)
                    || p.plugin_type.to_lowercase().contains(&query_lower)
                    || p.uri.to_lowercase().contains(&query_lower)
            })
            .filter_map(|p| {
                let params = self.get_plugin_parameters(&p.uri)?;
                Some((p, params))
            })
            .collect()
    }

    pub fn load_plugin(
        &mut self,
        uri: &str,
        sample_rate: f64,
    ) -> Result<(String, Vec<ParameterInfo>)> {
        let plugin = self
            .world
            .plugin_by_uri(uri)
            .ok_or_else(|| anyhow!("Plugin not found: {}", uri))?;

        let parameters = Self::extract_parameters(&plugin);
        let parameter_values: Vec<(String, f32)> = parameters
            .iter()
            .map(|p| (p.symbol.clone(), p.default))
            .collect();

        let instance = unsafe {
            plugin
                .instantiate(self.features.clone(), sample_rate)
                .map_err(|e| anyhow!("Failed to instantiate plugin: {:?}", e))?
        };

        let port_counts = instance.port_counts();

        let atom_sequence_inputs: Vec<LV2AtomSequence> = (0..port_counts.atom_sequence_inputs)
            .map(|_| LV2AtomSequence::new(&self.features, 1024))
            .collect();

        let atom_sequence_outputs: Vec<LV2AtomSequence> = (0..port_counts.atom_sequence_outputs)
            .map(|_| LV2AtomSequence::new(&self.features, 1024))
            .collect();

        let id = Uuid::new_v4().to_string();

        let info = self
            .available_plugins
            .iter()
            .find(|p| p.uri == uri)
            .cloned()
            .unwrap_or_else(|| PluginInfo {
                uri: uri.to_string(),
                name: uri.split('#').last().unwrap_or(uri).to_string(),
                plugin_type: "unknown".to_string(),
            });

        let active_plugin = ActivePlugin {
            id: id.clone(),
            instance,
            info,
            features: self.features.clone(),
            parameters: parameters.clone(),
            parameter_values,
            atom_sequence_inputs,
            atom_sequence_outputs,
        };

        self.plugins.push(active_plugin);
        Ok((id, parameters))
    }

    pub fn remove_plugin(&mut self, id: &str) -> Result<()> {
        let pos = self
            .plugins
            .iter()
            .position(|p| p.id == id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", id))?;

        self.plugins.remove(pos);
        Ok(())
    }

    pub fn set_parameter(&mut self, plugin_id: &str, param_name: &str, value: f32) -> Result<()> {
        let plugin = self
            .plugins
            .iter_mut()
            .find(|p| p.id == plugin_id)
            .ok_or_else(|| anyhow!("Plugin not found: {}", plugin_id))?;

        let param = plugin
            .parameters
            .iter()
            .find(|p| {
                p.name.to_lowercase() == param_name.to_lowercase()
                    || p.symbol.to_lowercase() == param_name.to_lowercase()
                    || p.name.to_lowercase().contains(&param_name.to_lowercase())
            })
            .ok_or_else(|| {
                let available: Vec<&str> =
                    plugin.parameters.iter().map(|p| p.name.as_str()).collect();
                anyhow!(
                    "Parameter '{}' not found. Available: {}",
                    param_name,
                    available.join(", ")
                )
            })?;

        let uri = plugin.info.uri.clone();
        let world_plugin = self
            .world
            .plugin_by_uri(&uri)
            .ok_or_else(|| anyhow!("Plugin not found in world"))?;

        for port in world_plugin.ports() {
            if port.symbol == param.symbol || port.name == param.name {
                if matches!(port.port_type, livi::PortType::ControlInput) {
                    let clamped = value.clamp(param.min, param.max);
                    plugin.instance.set_control_input(port.index, clamped);

                    for (sym, val) in plugin.parameter_values.iter_mut() {
                        if sym == &param.symbol {
                            *val = clamped;
                            break;
                        }
                    }
                    return Ok(());
                }
            }
        }

        Err(anyhow!("Parameter '{}' not found in plugin", param_name))
    }

    pub fn get_active_plugins(&self) -> &[ActivePlugin] {
        &self.plugins
    }

    pub fn get_plugin_by_id(&self, id: &str) -> Option<&ActivePlugin> {
        self.plugins.iter().find(|p| p.id == id)
    }

    pub fn export_config(&self, name: &str) -> ChainConfig {
        let plugins: Vec<PluginConfig> = self
            .plugins
            .iter()
            .map(|p| PluginConfig {
                uri: p.info.uri.clone(),
                name: p.info.name.clone(),
                parameters: p.parameter_values.clone(),
            })
            .collect();

        ChainConfig {
            version: "1.0".to_string(),
            name: name.to_string(),
            bypass: self.bypass,
            plugins,
        }
    }

    pub fn save_config(&self, name: &str, path: Option<&str>) -> Result<PathBuf> {
        let config = self.export_config(name);
        let config_path = path.map(PathBuf::from).unwrap_or_else(|| {
            let mut p = std::env::current_dir().unwrap_or_default();
            p.push(format!("{}.estima.json", name));
            p
        });

        let json = serde_json::to_string_pretty(&config)?;
        fs::write(&config_path, json)?;

        Ok(config_path)
    }

    pub fn load_config(&mut self, path: &str, sample_rate: f64) -> Result<ChainConfig> {
        let json = fs::read_to_string(path)?;
        let config: ChainConfig = serde_json::from_str(&json)?;

        self.plugins.clear();
        self.bypass = config.bypass;

        for plugin_config in &config.plugins {
            let result = self.load_plugin(&plugin_config.uri, sample_rate);
            if let Ok((id, _)) = result {
                for (param_symbol, value) in &plugin_config.parameters {
                    let _ = self.set_parameter(&id, param_symbol, *value);
                }
            }
        }

        Ok(config)
    }

    pub fn list_presets() -> Result<Vec<(String, PathBuf)>> {
        let dir = std::env::current_dir().unwrap_or_default();
        let entries: Vec<(String, PathBuf)> = fs::read_dir(&dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                e.path()
                    .extension()
                    .map(|ext| ext == "json")
                    .unwrap_or(false)
                    && e.file_name().to_string_lossy().ends_with(".estima.json")
            })
            .filter_map(|e| {
                let name = e
                    .file_name()
                    .to_string_lossy()
                    .strip_suffix(".estima.json")?
                    .to_string();
                Some((name, e.path()))
            })
            .collect();
        Ok(entries)
    }

    pub fn process(&mut self, input: &[f32], output: &mut [f32], nframes: usize) {
        output.copy_from_slice(input);

        if self.bypass {
            return;
        }

        for plugin in &mut self.plugins {
            let port_counts = plugin.instance.port_counts();
            let audio_in_count = port_counts.audio_inputs;
            let audio_out_count = port_counts.audio_outputs;

            let in_buf: Vec<f32> = output.to_vec();

            if audio_in_count == 2 && audio_out_count == 2 {
                let left: Vec<f32> = in_buf.iter().step_by(2).copied().collect();
                let right: Vec<f32> = in_buf.iter().skip(1).step_by(2).copied().collect();

                let mut out_left = vec![0.0f32; nframes];
                let mut out_right = vec![0.0f32; nframes];

                let ports = PortConnections {
                    audio_inputs: [left.as_slice(), right.as_slice()].into_iter(),
                    audio_outputs: [out_left.as_mut_slice(), out_right.as_mut_slice()].into_iter(),
                    atom_sequence_inputs: plugin.atom_sequence_inputs.iter(),
                    atom_sequence_outputs: plugin.atom_sequence_outputs.iter_mut(),
                    cv_inputs: std::iter::empty(),
                    cv_outputs: std::iter::empty(),
                };

                unsafe {
                    plugin.instance.run(nframes, ports).unwrap_or_else(|e| {
                        log::error!("Plugin processing error: {:?}", e);
                    });
                }

                for (i, (l, r)) in out_left.iter().zip(out_right.iter()).enumerate() {
                    if i * 2 < output.len() {
                        output[i * 2] = *l;
                        output[i * 2 + 1] = *r;
                    }
                }
            } else if audio_in_count == 1 && audio_out_count == 1 {
                let mono: Vec<f32> = in_buf.iter().step_by(2).copied().collect();
                let mut out_mono = vec![0.0f32; nframes];

                let ports = PortConnections {
                    audio_inputs: std::iter::once(mono.as_slice()),
                    audio_outputs: std::iter::once(out_mono.as_mut_slice()),
                    atom_sequence_inputs: plugin.atom_sequence_inputs.iter(),
                    atom_sequence_outputs: plugin.atom_sequence_outputs.iter_mut(),
                    cv_inputs: std::iter::empty(),
                    cv_outputs: std::iter::empty(),
                };

                unsafe {
                    plugin.instance.run(nframes, ports).unwrap_or_else(|e| {
                        log::error!("Plugin processing error: {:?}", e);
                    });
                }

                for (i, s) in out_mono.iter().enumerate() {
                    if i * 2 < output.len() {
                        output[i * 2] = *s;
                        output[i * 2 + 1] = *s;
                    }
                }
            } else if audio_in_count == 0 && audio_out_count >= 1 {
                let mut outputs: Vec<Vec<f32>> = (0..audio_out_count)
                    .map(|_| vec![0.0f32; nframes])
                    .collect();

                let ports = PortConnections {
                    audio_inputs: std::iter::empty(),
                    audio_outputs: outputs.iter_mut().map(|o| o.as_mut_slice()),
                    atom_sequence_inputs: plugin.atom_sequence_inputs.iter(),
                    atom_sequence_outputs: plugin.atom_sequence_outputs.iter_mut(),
                    cv_inputs: std::iter::empty(),
                    cv_outputs: std::iter::empty(),
                };

                unsafe {
                    plugin.instance.run(nframes, ports).unwrap_or_else(|e| {
                        log::error!("Plugin processing error: {:?}", e);
                    });
                }

                for (i, samples) in outputs.iter().enumerate() {
                    for (j, s) in samples.iter().enumerate() {
                        if j * 2 + i < output.len() {
                            if i == 0 {
                                output[j * 2] = *s;
                            } else if i == 1 {
                                output[j * 2 + 1] = *s;
                            }
                        }
                    }
                }

                if audio_out_count == 1 {
                    for j in 0..nframes {
                        if j * 2 + 1 < output.len() {
                            output[j * 2 + 1] = output[j * 2];
                        }
                    }
                }
            } else if audio_in_count >= 1 && audio_out_count == 0 {
                let inputs: Vec<Vec<f32>> = if audio_in_count == 2 {
                    vec![
                        in_buf.iter().step_by(2).copied().collect(),
                        in_buf.iter().skip(1).step_by(2).copied().collect(),
                    ]
                } else {
                    vec![in_buf.iter().step_by(2).copied().collect()]
                };

                let ports = PortConnections {
                    audio_inputs: inputs.iter().map(|i| i.as_slice()),
                    audio_outputs: std::iter::empty(),
                    atom_sequence_inputs: plugin.atom_sequence_inputs.iter(),
                    atom_sequence_outputs: plugin.atom_sequence_outputs.iter_mut(),
                    cv_inputs: std::iter::empty(),
                    cv_outputs: std::iter::empty(),
                };

                unsafe {
                    plugin.instance.run(nframes, ports).unwrap_or_else(|e| {
                        log::error!("Plugin processing error: {:?}", e);
                    });
                }
            }
        }
    }

    pub fn clear(&mut self) {
        self.plugins.clear();
    }
}
