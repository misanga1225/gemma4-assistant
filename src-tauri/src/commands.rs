use crate::AppState;
use futures::StreamExt;
use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage};
use tauri::{AppHandle, Emitter, State};

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
    let request = ChatMessageRequest::new(state.config.model.clone(), request_messages);

    let mut stream = state
        .ollama
        .send_chat_messages_stream(request)
        .await
        .map_err(|e| format!("ストリームエラー: {}", e))?;

    let mut full_response = String::new();
    let mut completed = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(res) => {
                let token = &res.message.content;
                full_response.push_str(token);
                let _ = app.emit("chat-token", token.clone());

                if res.done {
                    completed = true;
                    break;
                }
            }
            Err(_) => {
                let error_message = "ストリーム中エラーが発生しました".to_string();
                let _ = app.emit("chat-error", error_message.clone());
                return Err(error_message);
            }
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
    let _ = app.emit("chat-complete", ());

    Ok(())
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    Ok(serde_json::json!({
        "name": state.config.name,
    }))
}
