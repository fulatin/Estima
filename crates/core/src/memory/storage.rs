use anyhow::Result;
use std::fs;
use std::path::PathBuf;
use super::Conversation;

pub struct MemoryStorage {
    storage_dir: PathBuf,
    conversation_file: PathBuf,
}

impl MemoryStorage {
    pub fn new() -> Result<Self> {
        let storage_dir = dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("estima")
            .join("conversations");

        fs::create_dir_all(&storage_dir)?;

        let conversation_file = storage_dir.join("default.json");

        Ok(Self {
            storage_dir,
            conversation_file,
        })
    }

    #[doc(hidden)]
    pub fn with_path(storage_dir: PathBuf, conversation_file: PathBuf) -> Result<Self> {
        fs::create_dir_all(&storage_dir)?;
        Ok(Self {
            storage_dir,
            conversation_file,
        })
    }

    pub fn load(&self) -> Result<Conversation> {
        if !self.conversation_file.exists() {
            return Ok(Conversation::new());
        }

        let content = fs::read_to_string(&self.conversation_file)?;
        let conversation: Conversation = serde_json::from_str(&content)?;
        Ok(conversation)
    }

    pub fn save(&self, conversation: &Conversation) -> Result<()> {
        let content = serde_json::to_string_pretty(conversation)?;
        fs::write(&self.conversation_file, content)?;
        Ok(())
    }

    pub fn clear(&self) -> Result<()> {
        if self.conversation_file.exists() {
            fs::remove_file(&self.conversation_file)?;
        }
        Ok(())
    }

    pub fn storage_path(&self) -> &std::path::Path {
        &self.conversation_file
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn load_returns_new_conversation_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("nonexistent.json");
        let storage = MemoryStorage::with_path(dir.path().to_path_buf(), file.clone()).unwrap();
        let conv = storage.load().unwrap();
        assert!(conv.messages.is_empty());
        assert!(!conv.id.is_empty());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.json");
        let storage = MemoryStorage::with_path(dir.path().to_path_buf(), file).unwrap();

        let mut conv = Conversation::new();
        conv.add_message(super::super::MessageRole::User, "Hello, world!");
        storage.save(&conv).unwrap();

        let loaded = storage.load().unwrap();
        assert_eq!(loaded.id, conv.id);
        assert_eq!(loaded.messages.len(), 1);
        assert_eq!(loaded.messages[0].content, "Hello, world!");
    }

    #[test]
    fn clear_removes_file() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("data.json");
        let storage = MemoryStorage::with_path(dir.path().to_path_buf(), file.clone()).unwrap();

        let conv = Conversation::new();
        storage.save(&conv).unwrap();
        assert!(file.exists());

        storage.clear().unwrap();
        assert!(!file.exists());
    }

    #[test]
    fn clear_does_not_error_when_file_missing() {
        let dir = TempDir::new().unwrap();
        let file = dir.path().join("missing.json");
        let storage = MemoryStorage::with_path(dir.path().to_path_buf(), file).unwrap();
        assert!(storage.clear().is_ok());
    }
}

impl Default for MemoryStorage {
    fn default() -> Self {
        Self::new().expect("Failed to create memory storage")
    }
}
