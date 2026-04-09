use reqwest::Client;
use serde::Serialize;
use std::time::Duration;

#[derive(Clone)]
pub struct IrodoriTtsClient {
    client: Client,
    base_url: String,
}

#[derive(Serialize)]
struct SynthRequest<'a> {
    text: &'a str,
}

impl IrodoriTtsClient {
    pub fn new(base_url: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(120))
            .build()
            .expect("failed to build HTTP client");

        Self {
            client,
            base_url: base_url.trim_end_matches('/').to_string(),
        }
    }

    /// テキストからWAVバイナリを合成する（Modal上のIrodori-TTS）
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        let url = format!("{}/synthesize", self.base_url);
        let wav_bytes = self
            .client
            .post(&url)
            .json(&SynthRequest { text })
            .send()
            .await
            .map_err(|e| format!("Irodori-TTS接続エラー: {}", e))?
            .error_for_status()
            .map_err(|e| format!("Irodori-TTSサーバーエラー: {}", e))?
            .bytes()
            .await
            .map_err(|e| format!("Irodori-TTS読み取り失敗: {}", e))?;

        Ok(wav_bytes.to_vec())
    }
}
