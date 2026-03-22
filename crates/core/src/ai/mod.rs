pub mod provider;
pub mod ollama;
pub mod openai;

pub use provider::AIProvider;
pub use ollama::OllamaProvider;
pub use openai::{OpenAIProvider, OpenAICompatibleProvider, AIProviderType};
