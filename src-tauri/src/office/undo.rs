use std::path::{Path, PathBuf};

/// 編集前のファイルをバックアップ。tasks/edit_history/YYYYMMDD-HHMMSS-<filename> へコピー。
pub fn backup(target: &Path, history_root: &Path) -> Result<Option<PathBuf>, String> {
    if !target.exists() {
        return Ok(None);
    }
    std::fs::create_dir_all(history_root)
        .map_err(|e| format!("履歴ディレクトリ作成失敗: {}", e))?;

    let ts = chrono::Local::now().format("%Y%m%d-%H%M%S");
    let filename = target
        .file_name()
        .map(|f| f.to_string_lossy().to_string())
        .unwrap_or_else(|| "file".to_string());
    let dest = history_root.join(format!("{ts}-{filename}"));
    std::fs::copy(target, &dest)
        .map_err(|e| format!("バックアップ失敗: {}", e))?;
    Ok(Some(dest))
}

/// 最新のバックアップから復元。対象ファイル名と一致する最も新しいバックアップを戻す。
pub fn restore_latest(target: &Path, history_root: &Path) -> Result<PathBuf, String> {
    let name = target
        .file_name()
        .ok_or("ファイル名が取得できません")?
        .to_string_lossy()
        .to_string();
    let entries = std::fs::read_dir(history_root)
        .map_err(|e| format!("履歴読み取り失敗: {}", e))?;
    let mut candidates: Vec<PathBuf> = entries
        .filter_map(|r| r.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .map(|f| f.to_string_lossy().ends_with(&name))
                .unwrap_or(false)
        })
        .collect();
    candidates.sort();
    let latest = candidates
        .pop()
        .ok_or_else(|| format!("{} のバックアップが見つかりません", name))?;
    std::fs::copy(&latest, target)
        .map_err(|e| format!("復元コピー失敗: {}", e))?;
    Ok(latest)
}
