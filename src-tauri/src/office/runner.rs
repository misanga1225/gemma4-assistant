use std::process::Stdio;
use tokio::io::AsyncWriteExt;
use tokio::process::Command;

/// PowerShellでCOMスクリプトを実行し、stdoutを返す。
///
/// スクリプトは UTF-8 でstdin渡し。先頭に OutputEncoding 設定を自動注入する。
pub async fn run_ps(script: &str) -> Result<String, String> {
    let wrapped = format!(
        "$ErrorActionPreference = 'Stop'\n\
         [Console]::OutputEncoding = [System.Text.Encoding]::UTF8\n\
         $OutputEncoding = [System.Text.Encoding]::UTF8\n\
         try {{\n{script}\n}} catch {{\n    \
           Write-Error $_.Exception.Message\n    \
           exit 1\n\
         }}\n"
    );

    let mut child = Command::new("powershell")
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-ExecutionPolicy", "Bypass",
            "-Command", "-",
        ])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("powershell起動失敗: {}", e))?;

    if let Some(mut stdin) = child.stdin.take() {
        stdin
            .write_all(wrapped.as_bytes())
            .await
            .map_err(|e| format!("stdin書き込み失敗: {}", e))?;
        drop(stdin);
    }

    let output = child
        .wait_with_output()
        .await
        .map_err(|e| format!("powershell実行失敗: {}", e))?;

    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();

    if !output.status.success() {
        return Err(format!(
            "powershell失敗(code={:?}): {}",
            output.status.code(),
            stderr.trim()
        ));
    }
    Ok(stdout)
}

/// PowerShell文字列リテラル用エスケープ（シングルクォート内に入れる想定）。
pub fn ps_escape(s: &str) -> String {
    s.replace('\'', "''")
}
