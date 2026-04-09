use crate::tts::TtsEngine;
use crate::AppState;
use base64::Engine;
use futures::StreamExt;
use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage};
use tauri::{AppHandle, Emitter, State};

/// 文の区切り文字で分割するためのヘルパー
fn is_sentence_end(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '!' | '?' | '\n')
}

/// 文が完成したらTTS合成タスクをspawnする
fn spawn_synthesis(
    tts: TtsEngine,
    app: AppHandle,
    sentence: String,
    index: usize,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        match tts.synthesize(&sentence).await {
            Ok(wav_bytes) => {
                let b64 = base64::engine::general_purpose::STANDARD.encode(&wav_bytes);
                // index付きで送り、フロントエンドで順序再生
                let _ = app.emit(
                    "chat-voice",
                    serde_json::json!({
                        "index": index,
                        "audio": b64,
                    }),
                );
            }
            Err(e) => {
                eprintln!("[tts] 文#{} 合成失敗: {}", index, e);
            }
        }
    })
}

#[tauri::command]
pub async fn ping_ollama(state: State<'_, AppState>) -> Result<String, String> {
    match state.ollama.list_local_models().await {
        Ok(models) => {
            let names: Vec<String> = models.iter().map(|m| m.name.clone()).collect();
            Ok(format!("接続成功! モデル一覧: {:?}", names))
        }
        Err(e) => Err(format!("接続失敗: {}", e)),
    }
}

#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let user_msg = ChatMessage::user(message);
    let request_messages = {
        let history = state.history.lock().await;
        let mut messages = history.clone();
        messages.push(user_msg.clone());
        messages
    };
    let request =
        ChatMessageRequest::new(state.config.model.clone(), request_messages).think(false);
    let mut stream = state
        .ollama
        .send_chat_messages_stream(request)
        .await
        .map_err(|e| format!("ストリームエラー: {}", e))?;

    let mut full_response = String::new();
    let mut completed = false;

    // 文単位のTTS合成タスク管理
    let tts_available = state.tts.is_available().await;
    let mut sentence_buf = String::new();
    let mut sentence_index: usize = 0;
    let mut synth_tasks: Vec<tokio::task::JoinHandle<()>> = Vec::new();

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(res) => {
                let token = &res.message.content;
                if !token.is_empty() {
                    full_response.push_str(token);
                    let _ = app.emit("chat-token", token.clone());

                    // 文単位でTTS合成をキック
                    if tts_available {
                        sentence_buf.push_str(token);
                        if token.chars().any(is_sentence_end) {
                            let sentence = sentence_buf.trim().to_string();
                            if !sentence.is_empty() {
                                let task = spawn_synthesis(
                                    state.tts.clone(),
                                    app.clone(),
                                    sentence,
                                    sentence_index,
                                );
                                synth_tasks.push(task);
                                sentence_index += 1;
                            }
                            sentence_buf.clear();
                        }
                    }
                }

                if res.done {
                    completed = true;
                    break;
                }
            }
            Err(e) => {
                eprintln!("[send_message] chunk error: {:?}", e);
                let error_message = "ストリーム中エラーが発生しました".to_string();
                let _ = app.emit("chat-error", error_message.clone());
                return Err(error_message);
            }
        }
    }

    // 残りのバッファも合成
    if tts_available {
        let sentence = sentence_buf.trim().to_string();
        if !sentence.is_empty() {
            let task = spawn_synthesis(
                state.tts.clone(),
                app.clone(),
                sentence,
                sentence_index,
            );
            synth_tasks.push(task);
        }
    }

    if !completed {
        let error_message = "応答ストリームが途中で終了しました".to_string();
        let _ = app.emit("chat-error", error_message.clone());
        return Err(error_message);
    }

    {
        let mut history = state.history.lock().await;
        history.push(user_msg);
        history.push(ChatMessage::assistant(full_response));
    }

    // 合成完了通知（文の総数をフロントエンドに伝える）
    let _ = app.emit(
        "chat-complete",
        serde_json::json!({
            "voice_count": synth_tasks.len(),
        }),
    );

    // 合成タスクの完了を待つ（emitは各タスク内で行われる）
    for task in synth_tasks {
        let _ = task.await;
    }

    Ok(())
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": state.config.name,
    }))
}
