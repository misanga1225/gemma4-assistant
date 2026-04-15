use crate::browse::{self, BrowseIntent};
use crate::office::types::{EditAction, EditResult};
use crate::tools::parser;
use crate::triggers;
use crate::tts::TtsEngine;
use crate::AppState;
use base64::Engine;
use futures::StreamExt;
use ollama_rs::generation::chat::{request::ChatMessageRequest, ChatMessage};
use tauri::{AppHandle, Emitter, State};

fn is_sentence_end(c: char) -> bool {
    matches!(c, '。' | '！' | '？' | '!' | '?' | '\n')
}

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
                let _ = app.emit(
                    "chat-voice",
                    serde_json::json!({ "index": index, "audio": b64 }),
                );
            }
            Err(e) => eprintln!("[tts] 文#{} 合成失敗: {}", index, e),
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

/// チャット1ターンを実行。tool-callが検出されたらconfirm待ち or 即実行→再chatループ。
#[tauri::command]
pub async fn send_message(
    app: AppHandle,
    message: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let user_msg = ChatMessage::user(message.clone());
    {
        let mut hist = state.history.lock().await;
        hist.push(user_msg);
    }

    // キーワードトリガーを先に評価
    if state.config.office_enabled {
        if let Some(action) = triggers::evaluate(&message, &state.config.triggers) {
            eprintln!("[triggers] matched: {}", action.tool_name());
            return handle_tool_action(&app, &state, action).await;
        }
    }

    // ブラウジング前処理（既存のまま、ただしuser_msgはhistory末尾に入っている）
    let browse_context = if state.config.browse_enabled {
        match browse::detect_browse_intent(&message) {
            BrowseIntent::Url(url) => {
                let _ = app.emit("browse-status", serde_json::json!({"status": "fetching", "url": &url}));
                match browse::fetch_url(&url).await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        let _ = app.emit("browse-status", serde_json::json!({"status": "error", "message": &e}));
                        None
                    }
                }
            }
            BrowseIntent::Search(query) => {
                let _ = app.emit("browse-status", serde_json::json!({"status": "searching", "query": &query}));
                match browse::search_web(&query).await {
                    Ok(t) => Some(t),
                    Err(e) => {
                        let _ = app.emit("browse-status", serde_json::json!({"status": "error", "message": &e}));
                        None
                    }
                }
            }
            BrowseIntent::None => None,
        }
    } else {
        None
    };

    if let Some(ctx) = browse_context {
        let ctx_msg = ChatMessage::system(format!(
            "[最重要指示] ユーザーが検索や情報取得を依頼しました。以下のWeb検索結果を必ず参照し、その内容をもとに回答してください。\n\n[検索結果]\n{}\n[検索結果ここまで]",
            ctx
        ));
        let mut hist = state.history.lock().await;
        let last = hist.pop().unwrap();
        hist.push(ctx_msg);
        hist.push(last);
        let _ = app.emit("browse-status", serde_json::json!({"status": "done"}));
    }

    run_chat_turn(&app, &state, 0).await
}

/// 1回のchatラウンド。tool-callを検出したら再帰的に次ラウンドを回す。
async fn run_chat_turn(
    app: &AppHandle,
    state: &State<'_, AppState>,
    depth: u32,
) -> Result<(), String> {
    if depth >= state.config.max_tool_calls_per_turn {
        let _ = app.emit("chat-error", "ツール呼び出し上限に達しました".to_string());
        return Err("ツール呼び出し上限".into());
    }

    let request_messages = { state.history.lock().await.clone() };
    let request = ChatMessageRequest::new(state.config.model.clone(), request_messages).think(false);

    let mut stream = state
        .ollama
        .send_chat_messages_stream(request)
        .await
        .map_err(|e| format!("ストリームエラー: {}", e))?;

    let mut full = String::new();
    let mut completed = false;

    let tts_on = state.tts.is_available().await;
    let mut sentence_buf = String::new();
    let mut sent_idx: usize = 0;
    let mut synth_tasks = Vec::new();
    let mut in_tool_block = false;

    while let Some(chunk) = stream.next().await {
        match chunk {
            Ok(res) => {
                let token = &res.message.content;
                if !token.is_empty() {
                    full.push_str(token);

                    // tool-callブロック開始検出: バックティック + tool/json 宣言が出たら以降のtokenをユーザーに出さない
                    if !in_tool_block && (full.contains("```tool") || full.contains("```json") || full.contains("<tool_call>")) {
                        in_tool_block = true;
                    }

                    if !in_tool_block {
                        let _ = app.emit("chat-token", token.clone());
                        if tts_on {
                            sentence_buf.push_str(token);
                            if token.chars().any(is_sentence_end) {
                                let s = sentence_buf.trim().to_string();
                                if !s.is_empty() {
                                    synth_tasks.push(spawn_synthesis(
                                        state.tts.clone(), app.clone(), s, sent_idx,
                                    ));
                                    sent_idx += 1;
                                }
                                sentence_buf.clear();
                            }
                        }
                    }
                }
                if res.done { completed = true; break; }
            }
            Err(e) => {
                eprintln!("[send_message] chunk error: {:?}", e);
                let _ = app.emit("chat-error", "ストリーム中エラー".to_string());
                return Err("ストリームエラー".into());
            }
        }
    }

    if !completed {
        let _ = app.emit("chat-error", "応答が途中で終了".to_string());
        return Err("応答が途中で終了".into());
    }

    // tool-call抽出
    let tool_call = parser::extract_tool_call(&full);
    let visible = parser::strip_tool_call(&full);

    // 履歴にassistantメッセージを記録（ツールコール含む生出力）
    {
        let mut hist = state.history.lock().await;
        hist.push(ChatMessage::assistant(full.clone()));
    }

    // 残sentence合成（ツールブロックがなかった場合のみ）
    if tool_call.is_none() && tts_on {
        let s = sentence_buf.trim().to_string();
        if !s.is_empty() {
            synth_tasks.push(spawn_synthesis(state.tts.clone(), app.clone(), s, sent_idx));
        }
    }

    if let Some(action) = tool_call {
        eprintln!("[tool] detected: {}", action.tool_name());
        // tool実行中はTTSをキャンセル（中途半端な音声を残さない）
        for t in synth_tasks { t.abort(); }
        // 見える本文があれば先に出す
        if !visible.is_empty() {
            let _ = app.emit("chat-token", visible);
        }
        return Box::pin(handle_tool_action(app, state, action)).await;
    }

    let _ = app.emit(
        "chat-complete",
        serde_json::json!({ "voice_count": synth_tasks.len() }),
    );
    for t in synth_tasks { let _ = t.await; }
    Ok(())
}

/// tool-callを確認ダイアログ経由 or 即実行する。
async fn handle_tool_action(
    app: &AppHandle,
    state: &State<'_, AppState>,
    action: EditAction,
) -> Result<(), String> {
    if state.config.require_confirmation {
        {
            let mut pending = state.pending_tool.lock().await;
            *pending = Some(action.clone());
        }
        let _ = app.emit(
            "tool-confirm",
            serde_json::json!({
                "tool": action.tool_name(),
                "action": &action,
            }),
        );
        let _ = app.emit("chat-complete", serde_json::json!({ "voice_count": 0 }));
        return Ok(());
    }
    execute_and_continue(app, state, action, 0).await
}

async fn execute_and_continue(
    app: &AppHandle,
    state: &State<'_, AppState>,
    action: EditAction,
    depth: u32,
) -> Result<(), String> {
    let _ = app.emit("tool-status", serde_json::json!({"status": "running", "tool": action.tool_name()}));
    let result: EditResult = {
        let editor = state.office.lock().await;
        editor.execute(&action).await
    };
    let _ = app.emit("tool-status", serde_json::json!({"status": "done", "ok": result.ok, "message": &result.message}));

    // 結果をsystem messageとして履歴に追加 → 再chat
    let result_json = serde_json::to_string(&result).unwrap_or_default();
    {
        let mut hist = state.history.lock().await;
        hist.push(ChatMessage::system(format!(
            "[ツール実行結果] tool={} result={}",
            action.tool_name(),
            result_json
        )));
    }
    Box::pin(run_chat_turn(app, state, depth + 1)).await
}

#[tauri::command]
pub async fn confirm_tool_call(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let action = {
        let mut pending = state.pending_tool.lock().await;
        pending.take()
    };
    match action {
        Some(a) => execute_and_continue(&app, &state, a, 0).await,
        None => Err("保留中のtool-callがありません".into()),
    }
}

#[tauri::command]
pub async fn cancel_tool_call(
    app: AppHandle,
    state: State<'_, AppState>,
) -> Result<(), String> {
    {
        let mut pending = state.pending_tool.lock().await;
        *pending = None;
    }
    let mut hist = state.history.lock().await;
    hist.push(ChatMessage::system("[ツール実行キャンセル] ユーザーが拒否しました。自然に会話に戻ってください。".into()));
    drop(hist);
    run_chat_turn(&app, &state, 0).await
}

#[tauri::command]
pub async fn undo_last_edit(
    state: State<'_, AppState>,
    target: String,
) -> Result<String, String> {
    let editor = state.office.lock().await;
    let restored = editor.undo(&target)?;
    Ok(format!("復元元: {}", restored.display()))
}

#[tauri::command]
pub async fn get_config(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let avail = {
        let ed = state.office.lock().await;
        ed.available.clone()
    };
    Ok(serde_json::json!({
        "name": state.config.name,
        "office": {
            "word": avail.word,
            "excel": avail.excel,
            "powerpoint": avail.powerpoint,
        }
    }))
}
