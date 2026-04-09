mod commands;
mod config;
mod irodori;
pub mod tts;
pub mod voicevox;

use config::PersonalityConfig;
use irodori::IrodoriTtsClient;
use ollama_rs::generation::chat::ChatMessage;
use ollama_rs::Ollama;
use tauri::{
    menu::{MenuBuilder, MenuItemBuilder},
    tray::TrayIconBuilder,
    Manager,
};
use std::sync::Arc;
use tokio::sync::Mutex;
use tts::TtsEngine;
use voicevox::VoicevoxClient;

pub struct AppState {
    pub ollama: Ollama,
    pub history: Mutex<Vec<ChatMessage>>,
    pub config: PersonalityConfig,
    pub tts: TtsEngine,
}

pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            // personality.json の解決: CARGO_MANIFEST_DIR (src-tauri/) の親 (プロジェクトルート)
            let project_root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .parent()
                .expect("CARGO_MANIFEST_DIR has no parent")
                .to_path_buf();
            let config_path = project_root.join("personality.json");

            // 本番ビルド用フォールバック: resource_dir
            let config_path = if config_path.exists() {
                config_path
            } else if let Ok(resource_dir) = app.path().resource_dir() {
                resource_dir.join("personality.json")
            } else {
                config_path
            };

            let config = config::load_config_from_file(&config_path);
            let system_msg = ChatMessage::system(config.system_prompt.clone());

            // TTSエンジンの初期化
            let tts = match config.tts_engine.as_str() {
                "irodori" => {
                    let url = config.irodori_url.as_deref().unwrap_or_else(|| {
                        panic!("tts_engine が 'irodori' の場合、irodori_url の設定が必要です")
                    });
                    eprintln!("[tts] engine=irodori url={}", url);
                    TtsEngine::Irodori(Arc::new(IrodoriTtsClient::new(url)))
                }
                _ => {
                    let vv = Arc::new(Mutex::new(VoicevoxClient::new("http://localhost:50021")));
                    let vv_clone = vv.clone();
                    eprintln!("[tts] engine=voicevox");

                    // VOICEVOX speaker_id を非同期で解決
                    tauri::async_runtime::spawn(async move {
                        let mut client = vv_clone.lock().await;
                        match client.resolve_speaker_id("きりたん").await {
                            Ok(id) => eprintln!("[voicevox] speaker_id={}", id),
                            Err(e) => eprintln!("[voicevox] {}", e),
                        }
                    });

                    TtsEngine::Voicevox(vv)
                }
            };

            let state = AppState {
                ollama: Ollama::new("http://localhost".to_string(), 11434),
                history: Mutex::new(vec![system_msg]),
                config,
                tts,
            };
            app.manage(state);

            // mascotウィンドウを画面右下に配置
            if let Some(mascot) = app.get_webview_window("mascot") {
                if let Ok(Some(monitor)) = mascot.current_monitor() {
                    let screen = monitor.size();
                    let scale = monitor.scale_factor();
                    let x = (screen.width as f64 / scale) - 440.0;
                    let y = (screen.height as f64 / scale) - 410.0;
                    let _ = mascot.set_position(tauri::LogicalPosition::new(x, y));
                }
            }

            // トレイメニュー
            let show_mascot =
                MenuItemBuilder::with_id("show_mascot", "キャラクターを表示").build(app)?;
            let quit = MenuItemBuilder::with_id("quit", "終了").build(app)?;
            let menu = MenuBuilder::new(app)
                .items(&[&show_mascot, &quit])
                .build()?;

            TrayIconBuilder::new()
                .icon(app.default_window_icon().unwrap().clone())
                .menu(&menu)
                .on_menu_event(|app, event| match event.id().as_ref() {
                    "show_mascot" => {
                        if let Some(w) = app.get_webview_window("mascot") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                    "quit" => {
                        app.exit(0);
                    }
                    _ => {}
                })
                .on_tray_icon_event(|tray, event| {
                    if let tauri::tray::TrayIconEvent::Click { .. } = event {
                        if let Some(w) = tray.app_handle().get_webview_window("mascot") {
                            let _ = w.show();
                            let _ = w.set_focus();
                        }
                    }
                })
                .build(app)?;

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::ping_ollama,
            commands::send_message,
            commands::get_config,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
