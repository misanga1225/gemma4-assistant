use super::runner::{ps_escape, run_ps};
use super::types::EditResult;

/// Word.Application に接続 or 新規起動するPSスニペット。
fn word_header() -> &'static str {
    r#"
$wd = $null
try { $wd = [Runtime.InteropServices.Marshal]::GetActiveObject('Word.Application') } catch {}
if ($null -eq $wd) { $wd = New-Object -ComObject Word.Application }
$wd.Visible = $true
"#
}

fn open_or_create(path: &str) -> String {
    let p = ps_escape(path);
    format!(
        r#"
$target = '{p}'
$doc = $null
foreach ($d in $wd.Documents) {{ if ($d.FullName -eq $target) {{ $doc = $d; break }} }}
if ($null -eq $doc) {{
    if (Test-Path $target) {{ $doc = $wd.Documents.Open($target) }}
    else {{ $doc = $wd.Documents.Add(); $doc.SaveAs2($target) }}
}}
"#,
    )
}

pub async fn open(path: &str) -> EditResult {
    let script = format!("{}{}", word_header(), open_or_create(path));
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("Word文書を開きました: {path}")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn find_replace(
    path: &str,
    find: &str,
    replace: &str,
    match_case: bool,
) -> EditResult {
    let script = format!(
        r#"{header}{open}
$find = $doc.Content.Find
$find.ClearFormatting()
$find.Replacement.ClearFormatting()
$find.Text = '{find}'
$find.Replacement.Text = '{replace}'
$find.MatchCase = ${mc}
# wdReplaceAll = 2
[void]$find.Execute($null,$null,$null,$null,$null,$null,$null,$null,$null,$null,2)
$doc.Save()
Write-Output "replaced"
"#,
        header = word_header(),
        open = open_or_create(path),
        find = ps_escape(find),
        replace = ps_escape(replace),
        mc = match_case,
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("置換完了: '{find}' → '{replace}'")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn append_paragraph(path: &str, text: &str, style: Option<&str>) -> EditResult {
    let style_set = match style {
        Some(s) => format!(
            "$rng.Style = $wd.ActiveDocument.Styles.Item('{}')",
            ps_escape(s)
        ),
        None => String::new(),
    };
    let script = format!(
        r#"{header}{open}
$rng = $doc.Content
$rng.Collapse(0)  # wdCollapseEnd
$rng.InsertParagraphAfter()
$rng.Collapse(0)
$rng.Text = '{text}'
{style_set}
$doc.Save()
"#,
        header = word_header(),
        open = open_or_create(path),
        text = ps_escape(text),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok("段落を追加しました".to_string()),
        Err(e) => EditResult::err(e),
    }
}

pub async fn insert_heading(path: &str, text: &str, level: u8) -> EditResult {
    let level = level.clamp(1, 9);
    let script = format!(
        r#"{header}{open}
$rng = $doc.Content
$rng.Collapse(0)
$rng.InsertParagraphAfter()
$rng.Collapse(0)
$rng.Text = '{text}'
$rng.Style = $wd.ActiveDocument.Styles.Item('Heading {level}')
$doc.Save()
"#,
        header = word_header(),
        open = open_or_create(path),
        text = ps_escape(text),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("見出し(Heading {level})を挿入しました")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn save_as(path: &str, dest: &str) -> EditResult {
    let script = format!(
        r#"{header}{open}
$doc.SaveAs2('{dest}')
"#,
        header = word_header(),
        open = open_or_create(path),
        dest = ps_escape(dest),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("保存しました: {dest}")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn is_available() -> bool {
    let script = r#"
try {
    $app = New-Object -ComObject Word.Application
    $app.Quit()
    Write-Output "ok"
} catch { exit 1 }
"#;
    run_ps(script).await.is_ok()
}
