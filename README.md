# Gemma4 Assistant

## 概要

gemma4を使用したデスクトップ常駐型アシスタント。TTSエンジンは2種類から選択可能:

- **VoiceVox**（デフォルト） — ローカル実行，オフライン対応
- **Irodori-TTS** — Modal経由でGPU上で実行，声クローニング対応

## 起動

```bash
cargo tauri dev
```

## TTSエンジンの切り替え

デフォルトはVoiceVox
Irodori-TTS変更時は`personality.json` の `tts_engine` 以下のように変更（Modalセットアップ必要）

1. `modal_app/ref_wavs/default.wav` に声のサンプルWAVを配置
2. Modalにデプロイ:
   ```bash
   cd modal_app && modal deploy irodori_app.py
   ```
3. `personality.json` に設定:
   ```json
   {
     "tts_engine": "irodori",
     "irodori_url": "https://<your-workspace>"
   }
   ```
