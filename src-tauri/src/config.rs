use crate::triggers::TriggerRule;
use serde::Deserialize;
use std::path::{Path, PathBuf};

#[derive(Debug, Clone, Deserialize)]
pub struct PersonalityConfig {
    pub name: String,
    pub model: String,
    pub system_prompt: String,
    #[serde(default = "default_tts_engine")]
    pub tts_engine: String,
    #[serde(default)]
    pub irodori_url: Option<String>,
    #[serde(default = "default_true")]
    pub browse_enabled: bool,
    #[serde(default = "default_true")]
    pub office_enabled: bool,
    #[serde(default)]
    pub office_whitelist: Vec<PathBuf>,
    #[serde(default = "default_true")]
    pub require_confirmation: bool,
    #[serde(default = "default_max_tool_calls")]
    pub max_tool_calls_per_turn: u32,
    #[serde(default)]
    pub triggers: Vec<TriggerRule>,
}

fn default_tts_engine() -> String { "voicevox".to_string() }
fn default_true() -> bool { true }
fn default_max_tool_calls() -> u32 { 5 }

impl Default for PersonalityConfig {
    fn default() -> Self {
        Self {
            name: "アシスタント".to_string(),
            model: "gemma4".to_string(),
            system_prompt: "あなたは優しいアシスタントです。日本語で会話してください。".to_string(),
            tts_engine: default_tts_engine(),
            irodori_url: None,
            browse_enabled: true,
            office_enabled: true,
            office_whitelist: Vec::new(),
            require_confirmation: true,
            max_tool_calls_per_turn: default_max_tool_calls(),
            triggers: Vec::new(),
        }
    }
}

pub fn load_config_from_file(path: &Path) -> PersonalityConfig {
    eprintln!("[config] loading: {}", path.display());
    match std::fs::read_to_string(path) {
        Ok(content) => serde_json::from_str(&content).unwrap_or_else(|e| {
            eprintln!("personality.json のパースに失敗: {}。デフォルトを使用します。", e);
            PersonalityConfig::default()
        }),
        Err(_) => {
            eprintln!("personality.json が見つかりません ({})。デフォルトを使用します。", path.display());
            PersonalityConfig::default()
        }
    }
}
