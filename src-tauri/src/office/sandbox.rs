use std::path::{Path, PathBuf};

/// ホワイトリスト配下かチェック。空なら全許可（後方互換・デフォルト）。
pub fn check_allowed(target: &str, whitelist: &[PathBuf]) -> Result<PathBuf, String> {
    let target_abs = Path::new(target);
    let target_abs = if target_abs.is_absolute() {
        target_abs.to_path_buf()
    } else {
        std::env::current_dir()
            .map_err(|e| format!("cwd取得失敗: {}", e))?
            .join(target_abs)
    };

    if whitelist.is_empty() {
        return Ok(target_abs);
    }

    let canon_target = target_abs.canonicalize().unwrap_or(target_abs.clone());
    for root in whitelist {
        let canon_root = root.canonicalize().unwrap_or_else(|_| root.clone());
        if canon_target.starts_with(&canon_root) {
            return Ok(target_abs);
        }
    }
    Err(format!(
        "サンドボックス拒否: {} はホワイトリスト外です",
        target_abs.display()
    ))
}
