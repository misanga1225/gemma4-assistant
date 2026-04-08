use reqwest::Client;
use serde::{Deserialize, Serialize};

#[derive(Clone)]
pub struct VoicevoxClient {
    client: Client,
    base_url: String,
    pub speaker_id: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct Speaker {
    name: String,
    styles: Vec<SpeakerStyle>,
}

#[derive(Debug, Deserialize)]
struct SpeakerStyle {
    id: u32,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AudioQuery(serde_json::Value);

impl VoicevoxClient {
    pub fn new(base_url: &str) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.to_string(),
            speaker_id: None,
        }
    }

    /// /speakers からキャラ名でspeaker_idを解決する
    pub async fn resolve_speaker_id(&mut self, name: &str) -> Result<u32, String> {
        let url = format!("{}/speakers", self.base_url);
        let speakers: Vec<Speaker> = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("VOICEVOX接続失敗: {}", e))?
            .json()
            .await
            .map_err(|e| format!("VOICEVOX応答パース失敗: {}", e))?;

        for speaker in &speakers {
            if speaker.name.contains(name) {
                if let Some(style) = speaker.styles.first() {
                    self.speaker_id = Some(style.id);
                    return Ok(style.id);
                }
            }
        }
        Err(format!("スピーカー '{}' が見つかりません", name))
    }

    /// テキストからWAVバイナリを合成する（2ステップ）
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        let speaker_id = self
            .speaker_id
            .ok_or("speaker_idが未設定です".to_string())?;

        // Step 1: audio_query
        let query_url = format!(
            "{}/audio_query?text={}&speaker={}",
            self.base_url,
            urlencoded(text),
            speaker_id
        );
        let audio_query: AudioQuery = self
            .client
            .post(&query_url)
            .send()
            .await
            .map_err(|e| format!("audio_queryエラー: {}", e))?
            .json()
            .await
            .map_err(|e| format!("audio_queryパース失敗: {}", e))?;

        // Step 2: synthesis
        let synth_url = format!("{}/synthesis?speaker={}", self.base_url, speaker_id);
        let wav_bytes = self
            .client
            .post(&synth_url)
            .json(&audio_query.0)
            .send()
            .await
            .map_err(|e| format!("synthesisエラー: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("synthesis読み取り失敗: {}", e))?;

        Ok(wav_bytes.to_vec())
    }
}

fn urlencoded(s: &str) -> String {
    s.chars()
        .flat_map(|c| {
            let mut buf = [0u8; 4];
            let encoded = c.encode_utf8(&mut buf);
            encoded
                .bytes()
                .map(|b| match b {
                    b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                        format!("{}", b as char)
                    }
                    _ => format!("%{:02X}", b),
                })
                .collect::<Vec<_>>()
        })
        .collect()
}
