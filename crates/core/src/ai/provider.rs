use async_trait::async_trait;
use anyhow::Result;

#[async_trait]
pub trait AIProvider: Send + Sync {
    async fn chat(&self, message: &str, system_prompt: Option<&str>) -> Result<String>;
    fn provider_name(&self) -> &str;
}
