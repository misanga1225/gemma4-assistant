use super::runner::{ps_escape, run_ps};
use super::types::EditResult;

fn pp_header() -> &'static str {
    r#"
$pp = $null
try { $pp = [Runtime.InteropServices.Marshal]::GetActiveObject('PowerPoint.Application') } catch {}
if ($null -eq $pp) { $pp = New-Object -ComObject PowerPoint.Application }
$pp.Visible = $true
"#
}

fn open_or_create(path: &str) -> String {
    let p = ps_escape(path);
    format!(
        r#"
$target = '{p}'
$pres = $null
foreach ($q in $pp.Presentations) {{ if ($q.FullName -eq $target) {{ $pres = $q; break }} }}
if ($null -eq $pres) {{
    if (Test-Path $target) {{ $pres = $pp.Presentations.Open($target) }}
    else {{ $pres = $pp.Presentations.Add(); $pres.SaveAs($target) }}
}}
"#
    )
}

pub async fn add_slide(path: &str, title: Option<&str>, body: Option<&str>) -> EditResult {
    let title = title.unwrap_or("");
    let body = body.unwrap_or("");
    let script = format!(
        r#"{header}{open}
# ppLayoutText = 2
$index = $pres.Slides.Count + 1
$slide = $pres.Slides.Add($index, 2)
if ('{title}' -ne '') {{ $slide.Shapes.Title.TextFrame.TextRange.Text = '{title}' }}
if ('{body}' -ne '' -and $slide.Shapes.Count -ge 2) {{ $slide.Shapes.Item(2).TextFrame.TextRange.Text = '{body}' }}
$pres.Save()
"#,
        header = pp_header(),
        open = open_or_create(path),
        title = ps_escape(title),
        body = ps_escape(body),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok("スライドを追加しました".to_string()),
        Err(e) => EditResult::err(e),
    }
}

pub async fn edit_text(
    path: &str,
    slide_index: i32,
    shape_index: i32,
    text: &str,
) -> EditResult {
    let script = format!(
        r#"{header}{open}
$slide = $pres.Slides.Item({slide_index})
$shape = $slide.Shapes.Item({shape_index})
$shape.TextFrame.TextRange.Text = '{text}'
$pres.Save()
"#,
        header = pp_header(),
        open = open_or_create(path),
        text = ps_escape(text),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("Slide#{slide_index} Shape#{shape_index} のテキストを更新")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn is_available() -> bool {
    let script = r#"
try {
    $app = New-Object -ComObject PowerPoint.Application
    $app.Quit()
    Write-Output "ok"
} catch { exit 1 }
"#;
    run_ps(script).await.is_ok()
}
