use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct AIConfig {
    pub provider: String,
    pub api_key: String,
    pub model: Option<String>,
    pub base_url: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    pub ai: AIConfig,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            ai: AIConfig {
                provider: String::new(),
                api_key: String::new(),
                model: None,
                base_url: None,
            },
        }
    }
}

impl AppConfig {
    pub fn config_dir() -> PathBuf {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("estima")
    }

    pub fn config_path() -> PathBuf {
        Self::config_dir().join("config.json")
    }

    pub fn load() -> Result<Self> {
        let path = Self::config_path();
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = fs::read_to_string(&path)?;
        let config: AppConfig = serde_json::from_str(&content)?;
        Ok(config)
    }

    pub fn save(&self) -> Result<()> {
        let dir = Self::config_dir();
        if !dir.exists() {
            fs::create_dir_all(&dir)?;
        }
        let content = serde_json::to_string_pretty(self)?;
        fs::write(Self::config_path(), content)?;
        Ok(())
    }

    pub fn is_configured(&self) -> bool {
        !self.ai.provider.is_empty() && !self.ai.api_key.is_empty()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config() {
        let config = AppConfig::default();
        assert!(!config.is_configured());
    }

    #[test]
    fn is_configured() {
        let mut config = AppConfig::default();
        assert!(!config.is_configured());
        
        config.ai.provider = "deepseek".to_string();
        assert!(!config.is_configured());
        
        config.ai.api_key = "test-key".to_string();
        assert!(config.is_configured());
    }
}
