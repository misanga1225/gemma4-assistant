use crate::irodori::IrodoriTtsClient;
use crate::voicevox::VoicevoxClient;
use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Clone)]
pub enum TtsEngine {
    Voicevox(Arc<Mutex<VoicevoxClient>>),
    Irodori(Arc<IrodoriTtsClient>),
}

impl TtsEngine {
    pub async fn synthesize(&self, text: &str) -> Result<Vec<u8>, String> {
        match self {
            TtsEngine::Voicevox(vv) => {
                let client = vv.lock().await;
                client.synthesize(text).await
            }
            TtsEngine::Irodori(client) => client.synthesize(text).await,
        }
    }

    pub async fn is_available(&self) -> bool {
        match self {
            TtsEngine::Voicevox(vv) => {
                let client = vv.lock().await;
                client.speaker_id.is_some()
            }
            TtsEngine::Irodori(_) => true,
        }
    }
}
