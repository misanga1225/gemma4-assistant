use crate::office::types::EditAction;
use regex::Regex;

/// LLM出力から tool-call JSONブロックを抽出する。
/// サポート形式: ```tool ... ```, ```json ... ```, <tool_call>...</tool_call>, 生JSON（先頭 `{"tool":`）
pub fn extract_tool_call(text: &str) -> Option<EditAction> {
    let patterns = [
        r"```tool\s*([\s\S]*?)```",
        r"```json\s*([\s\S]*?)```",
        r"<tool_call>\s*([\s\S]*?)</tool_call>",
    ];
    for pat in &patterns {
        if let Ok(re) = Regex::new(pat) {
            if let Some(caps) = re.captures(text) {
                if let Some(m) = caps.get(1) {
                    if let Some(action) = parse_tool_json(m.as_str().trim()) {
                        return Some(action);
                    }
                }
            }
        }
    }
    // 生JSONフォールバック
    if let Some(start) = text.find(r#"{"tool""#) {
        let rest = &text[start..];
        if let Some(end) = find_json_end(rest) {
            if let Some(action) = parse_tool_json(&rest[..=end]) {
                return Some(action);
            }
        }
    }
    None
}

fn find_json_end(s: &str) -> Option<usize> {
    let mut depth = 0i32;
    let mut in_str = false;
    let mut esc = false;
    for (i, ch) in s.char_indices() {
        if in_str {
            if esc { esc = false; continue; }
            match ch { '\\' => esc = true, '"' => in_str = false, _ => {} }
            continue;
        }
        match ch {
            '"' => in_str = true,
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
    }
    None
}

fn parse_tool_json(raw: &str) -> Option<EditAction> {
    let v: serde_json::Value = serde_json::from_str(raw).ok()?;
    let tool = v.get("tool")?.as_str()?;
    let args = v.get("args").cloned().unwrap_or(serde_json::json!({}));
    let merged = match args {
        serde_json::Value::Object(mut m) => {
            m.insert("tool".into(), serde_json::Value::String(tool.to_string()));
            serde_json::Value::Object(m)
        }
        _ => return None,
    };
    serde_json::from_value(merged).ok()
}

/// LLM出力からtool-callブロックを除去してユーザー向け本文だけ返す。
pub fn strip_tool_call(text: &str) -> String {
    let patterns = [
        r"```tool\s*[\s\S]*?```",
        r"```json\s*[\s\S]*?```",
        r"<tool_call>[\s\S]*?</tool_call>",
    ];
    let mut out = text.to_string();
    for pat in &patterns {
        if let Ok(re) = Regex::new(pat) {
            out = re.replace_all(&out, "").to_string();
        }
    }
    out.trim().to_string()
}
