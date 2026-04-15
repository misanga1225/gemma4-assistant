use super::runner::{ps_escape, run_ps};
use super::types::EditResult;

fn excel_header() -> &'static str {
    r#"
$xl = $null
try { $xl = [Runtime.InteropServices.Marshal]::GetActiveObject('Excel.Application') } catch {}
if ($null -eq $xl) { $xl = New-Object -ComObject Excel.Application }
$xl.Visible = $true
$xl.DisplayAlerts = $false
"#
}

fn open_or_create(path: &str) -> String {
    let p = ps_escape(path);
    format!(
        r#"
$target = '{p}'
$wb = $null
foreach ($b in $xl.Workbooks) {{ if ($b.FullName -eq $target) {{ $wb = $b; break }} }}
if ($null -eq $wb) {{
    if (Test-Path $target) {{ $wb = $xl.Workbooks.Open($target) }}
    else {{ $wb = $xl.Workbooks.Add(); $wb.SaveAs($target) }}
}}
"#
    )
}

fn select_sheet(sheet: &str) -> String {
    format!("$sh = $wb.Worksheets.Item('{}')\n", ps_escape(sheet))
}

pub async fn open(path: &str) -> EditResult {
    let script = format!("{}{}", excel_header(), open_or_create(path));
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("Excelブックを開きました: {path}")),
        Err(e) => EditResult::err(e),
    }
}

pub async fn read_range(path: &str, sheet: &str, range: &str) -> EditResult {
    let script = format!(
        r#"{header}{open}{sh}
$rng = $sh.Range('{range}')
$data = @()
if ($rng.Cells.Count -eq 1) {{
    $data = @(@($rng.Value2))
}} else {{
    foreach ($row in $rng.Rows) {{
        $line = @()
        foreach ($c in $row.Cells) {{ $line += ,$c.Value2 }}
        $data += ,$line
    }}
}}
$data | ConvertTo-Json -Compress -Depth 4
"#,
        header = excel_header(),
        open = open_or_create(path),
        sh = select_sheet(sheet),
        range = ps_escape(range),
    );
    match run_ps(&script).await {
        Ok(out) => {
            let trimmed = out.trim();
            let data = serde_json::from_str::<serde_json::Value>(trimmed)
                .unwrap_or_else(|_| serde_json::Value::String(trimmed.to_string()));
            EditResult::ok_data(format!("{}!{} を読みました", sheet, range), data)
        }
        Err(e) => EditResult::err(e),
    }
}

pub async fn write_cell(path: &str, sheet: &str, cell: &str, value: &str) -> EditResult {
    let script = format!(
        r#"{header}{open}{sh}
$sh.Range('{cell}').Value2 = '{value}'
$wb.Save()
"#,
        header = excel_header(),
        open = open_or_create(path),
        sh = select_sheet(sheet),
        cell = ps_escape(cell),
        value = ps_escape(value),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("{}!{} に書き込み完了", sheet, cell)),
        Err(e) => EditResult::err(e),
    }
}

pub async fn write_range(
    path: &str,
    sheet: &str,
    range: &str,
    values: &[Vec<String>],
) -> EditResult {
    let rows = values.len();
    let cols = values.iter().map(|r| r.len()).max().unwrap_or(0);
    if rows == 0 || cols == 0 {
        return EditResult::err("valuesが空です".to_string());
    }
    let mut ps_rows = String::new();
    for (i, row) in values.iter().enumerate() {
        for (j, v) in row.iter().enumerate() {
            ps_rows.push_str(&format!(
                "$arr[{i},{j}] = '{}'\n",
                ps_escape(v)
            ));
        }
    }
    let script = format!(
        r#"{header}{open}{sh}
$arr = New-Object 'object[,]' {rows},{cols}
{ps_rows}
$sh.Range('{range}').Value2 = $arr
$wb.Save()
"#,
        header = excel_header(),
        open = open_or_create(path),
        sh = select_sheet(sheet),
        range = ps_escape(range),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("{}!{} に {}×{} 書き込み完了", sheet, range, rows, cols)),
        Err(e) => EditResult::err(e),
    }
}

pub async fn add_formula(path: &str, sheet: &str, cell: &str, formula: &str) -> EditResult {
    let script = format!(
        r#"{header}{open}{sh}
$sh.Range('{cell}').Formula = '{formula}'
$wb.Save()
"#,
        header = excel_header(),
        open = open_or_create(path),
        sh = select_sheet(sheet),
        cell = ps_escape(cell),
        formula = ps_escape(formula),
    );
    match run_ps(&script).await {
        Ok(_) => EditResult::ok(format!("{}!{} に数式を設定", sheet, cell)),
        Err(e) => EditResult::err(e),
    }
}

pub async fn is_available() -> bool {
    let script = r#"
try {
    $app = New-Object -ComObject Excel.Application
    $app.Quit()
    Write-Output "ok"
} catch { exit 1 }
"#;
    run_ps(script).await.is_ok()
}
