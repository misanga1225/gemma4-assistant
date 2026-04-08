use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Clone, Deserialize)]
pub struct PersonalityConfig {
    pub name: String,
    pub model: String,
    pub system_prompt: String,
}

impl Default for PersonalityConfig {
    fn default() -> Self {
        Self {
            name: "アシスタント".to_string(),
            model: "gemma4".to_string(),
            system_prompt: "あなたは優しいアシスタントです。日本語で会話してください。".to_string(),
        }
    }
}

pub fn load_config_from_file(path: &Path) -> PersonalityConfig {
    eprintln!("[config] loading: {}", path.display());
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!(
                "personality.json のパースに失敗: {}。デフォルトを使用します。",
                e
            );
            PersonalityConfig::default()
        }),
        Err(_) => {
            eprintln!(
                "personality.json が見つかりません ({})。デフォルトを使用します。",
                path.display()
            );
            PersonalityConfig::default()
        }
    }
}
