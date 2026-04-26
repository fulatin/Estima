pub mod jack_engine;
pub mod plugin_chain;

#[cfg(feature = "plugin-ui")]
pub mod lv2_external_ui;

#[cfg(feature = "plugin-ui")]
pub mod carla_ui;

#[cfg(feature = "plugin-ui")]
pub mod plugin_ui;

pub use jack_engine::{AudioState, JackEngine};
pub use plugin_chain::{
    ActivePlugin, ChainConfig, ParameterInfo, PluginChain, PluginConfig, PluginInfo,
};

#[cfg(feature = "plugin-ui")]
pub use lv2_external_ui::LV2ExternalUIManager;

#[cfg(feature = "plugin-ui")]
pub use carla_ui::CarlaUIManager;

#[cfg(feature = "plugin-ui")]
pub use plugin_ui::{PluginUIInfo, PluginUIManager};
