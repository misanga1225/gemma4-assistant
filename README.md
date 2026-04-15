# Gemma4 Assistant

## 概要

gemma4を使用したデスクトップ常駐型アシスタント。TTSエンジンは2種類から選択可能:

- **VoiceVox**（デフォルト） — ローカル実行，オフライン対応
- **Irodori-TTS** — Modal経由でGPU上で実行，声クローニング対応

さらに Windows環境では **Word / Excel / PowerPoint の編集エージェント機能** を内蔵。

## 起動

```bash
cargo tauri dev
```

## TTSエンジンの切り替え

デフォルトはVoiceVox。Irodori-TTS変更時は`personality.json` の `tts_engine` 以下のように変更（Modalセットアップ必要）

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

## Office編集エージェント（Windowsのみ）

起動時にMS Officeの可用性を自動検出し、Gemma4にツール使用方法を注入します。
ユーザーが自然言語で編集を依頼すると、LLMが `tool-call` を出力 → 確認ダイアログ → 実行。

### 利用可能ツール

- Word: `word_open`, `word_find_replace`, `word_append_paragraph`, `word_insert_heading`, `word_save_as`
- Excel: `excel_open`, `excel_read_range`, `excel_write_cell`, `excel_write_range`, `excel_add_formula`
- PowerPoint: `pptx_add_slide`, `pptx_edit_text`

### 設定項目（personality.json）

| 項目 | 型 | 説明 |
|------|----|------|
| `office_enabled` | bool | 機能の有効化（デフォルト `true`） |
| `office_whitelist` | string[] | 編集許可ディレクトリ。空なら全許可 |
| `require_confirmation` | bool | 編集前に確認ダイアログを出す（デフォルト `true`） |
| `max_tool_calls_per_turn` | u32 | 1ターンでの連続ツール呼び出し上限（デフォルト `5`） |
| `triggers` | array | キーワード正規表現→ツールのショートカット宣言 |

### triggers の書き方

```json
"triggers": [
  {
    "pattern": "「(.+?)」を(.+?)に置換",
    "tool": "word_find_replace",
    "args": {
      "path": "C:\\docs\\report.docx",
      "find": "{{1}}",
      "replace": "{{2}}"
    }
  }
]
```

`{{1}}` `{{2}}` ... は正規表現のキャプチャグループで置換されます。

### 安全設計

- **サンドボックス**: `office_whitelist` 外は拒否
- **バックアップ**: 編集前に `tasks/edit_history/YYYYMMDD-HHMMSS-<filename>` へコピー
- **Undo**: `invoke('undo_last_edit', {target: "<path>"})` で最新バックアップから復元
- **暴走防止**: 1ターン内の連続ツール呼び出しに上限あり

### 内部実装

Office操作はPowerShell経由のCOMオートメーションで実装されています（`src-tauri/src/office/`）。
Office未インストール・非Windows環境では自動的に機能無効化されます。
