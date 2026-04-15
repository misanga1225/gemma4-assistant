use crate::office::types::EditAction;
use regex::Regex;
use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct TriggerRule {
    pub pattern: String,
    pub tool: String,
    #[serde(default)]
    pub args: serde_json::Value,
}

/// personality.json の triggers[] を評価してヒットしたらEditActionを返す。
/// args内の `{{1}}` `{{2}}` ... はキャプチャグループで置換される。
pub fn evaluate(message: &str, rules: &[TriggerRule]) -> Option<EditAction> {
    for rule in rules {
        let re = match Regex::new(&rule.pattern) {
            Ok(r) => r,
            Err(e) => {
                eprintln!("[triggers] invalid regex '{}': {}", rule.pattern, e);
                continue;
            }
        };
        if let Some(caps) = re.captures(message) {
            let mut args = rule.args.clone();
            substitute_captures(&mut args, &caps);
            let full = serde_json::json!({
                "tool": rule.tool,
                "args": args,
            });
            if let Ok(a) = action_from_value(full) {
                return Some(a);
            }
        }
    }
    None
}

fn substitute_captures(val: &mut serde_json::Value, caps: &regex::Captures) {
    match val {
        serde_json::Value::String(s) => {
            let mut out = s.clone();
            for i in 1..caps.len() {
                if let Some(m) = caps.get(i) {
                    out = out.replace(&format!("{{{{{}}}}}", i), m.as_str());
                }
            }
            *s = out;
        }
        serde_json::Value::Array(arr) => {
            for v in arr.iter_mut() { substitute_captures(v, caps); }
        }
        serde_json::Value::Object(m) => {
            for (_, v) in m.iter_mut() { substitute_captures(v, caps); }
        }
        _ => {}
    }
}

fn action_from_value(v: serde_json::Value) -> Result<EditAction, String> {
    let tool = v.get("tool").and_then(|t| t.as_str()).ok_or("tool欠損")?.to_string();
    let args = v.get("args").cloned().unwrap_or(serde_json::json!({}));
    let merged = match args {
        serde_json::Value::Object(mut m) => {
            m.insert("tool".into(), serde_json::Value::String(tool));
            serde_json::Value::Object(m)
        }
        _ => return Err("argsがobjectではない".into()),
    };
    serde_json::from_value(merged).map_err(|e| e.to_string())
}
