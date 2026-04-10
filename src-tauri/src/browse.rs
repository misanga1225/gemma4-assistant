use regex::Regex;
use reqwest::Client;
use scraper::{Html, Selector};
use std::time::Duration;

#[derive(Debug)]
pub enum BrowseIntent {
    None,
    Url(String),
    Search(String),
}

/// ユーザーメッセージからブラウジング意図を検出する
pub fn detect_browse_intent(message: &str) -> BrowseIntent {
    // URL検出
    let url_re = Regex::new(r"https?://[^\s]+").unwrap();
    if let Some(m) = url_re.find(message) {
        return BrowseIntent::Url(m.as_str().to_string());
    }

    // 検索パターン検出
    let search_patterns = [
        r"「(.+?)」を(?:検索|けんさく)して",
        r"「(.+?)」について調べて",
        r"「(.+?)」をググって",
        r"(.+?)を(?:検索|けんさく)して",
        r"(.+?)について調べて",
        r"(.+?)をググって",
        r"(?:検索|けんさく)して[：:]\s*(.+)",
    ];

    for pattern in &search_patterns {
        if let Ok(re) = Regex::new(pattern) {
            if let Some(caps) = re.captures(message) {
                if let Some(query) = caps.get(1) {
                    let q = query.as_str().trim().to_string();
                    if !q.is_empty() {
                        return BrowseIntent::Search(q);
                    }
                }
            }
        }
    }

    BrowseIntent::None
}

fn build_client() -> Result<Client, String> {
    Client::builder()
        .timeout(Duration::from_secs(10))
        .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36")
        .build()
        .map_err(|e| format!("HTTPクライアント作成失敗: {}", e))
}

/// URLからテキストを抽出する
pub async fn fetch_url(url: &str) -> Result<String, String> {
    eprintln!("[browse] fetching URL: {}", url);
    let client = build_client()?;
    let resp = client
        .get(url)
        .send()
        .await
        .map_err(|e| format!("リクエスト失敗: {}", e))?
        .error_for_status()
        .map_err(|e| format!("HTTPエラー: {}", e))?;

    let html = resp
        .text()
        .await
        .map_err(|e| format!("レスポンス読み取り失敗: {}", e))?;

    let text = extract_text_from_html(&html);
    if text.is_empty() {
        return Err("ページからテキストを抽出できませんでした".to_string());
    }
    Ok(truncate_to_limit(&text, 4000))
}

/// DuckDuckGo HTMLでWeb検索する
pub async fn search_web(query: &str) -> Result<String, String> {
    eprintln!("[browse] searching: {}", query);
    let client = build_client()?;
    let url = format!(
        "https://html.duckduckgo.com/html/?q={}",
        urlencoding(query)
    );

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("検索リクエスト失敗: {}", e))?
        .error_for_status()
        .map_err(|e| format!("検索HTTPエラー: {}", e))?;

    let html = resp
        .text()
        .await
        .map_err(|e| format!("検索レスポンス読み取り失敗: {}", e))?;

    let results = extract_search_results(&html);
    if results.is_empty() {
        return Err("検索結果が見つかりませんでした".to_string());
    }
    Ok(truncate_to_limit(&results, 4000))
}

/// 簡易URLエンコーディング
fn urlencoding(s: &str) -> String {
    let mut result = String::new();
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                result.push(byte as char);
            }
            _ => {
                result.push_str(&format!("%{:02X}", byte));
            }
        }
    }
    result
}

/// HTMLからメインコンテンツのテキストを抽出する
fn extract_text_from_html(html: &str) -> String {
    let document = Html::parse_document(html);

    let content_selector =
        Selector::parse("p, h1, h2, h3, h4, h5, h6, li, td, th, blockquote")
            .unwrap();

    let skip_tags: std::collections::HashSet<&str> =
        ["script", "style", "nav", "footer", "header", "noscript"]
            .iter()
            .copied()
            .collect();

    let mut texts = Vec::new();
    for element in document.select(&content_selector) {
        // 親要素にskip対象タグがあればスキップ
        let skip = element.ancestors().filter_map(|node| {
            scraper::ElementRef::wrap(node)
        }).any(|ancestor| {
            skip_tags.contains(ancestor.value().name())
        });
        if skip {
            continue;
        }

        let text: String = element.text().collect::<Vec<_>>().join(" ");
        let text = text.trim().to_string();
        if !text.is_empty() && text.len() > 1 {
            texts.push(text);
        }
    }

    texts.join("\n")
}

/// DuckDuckGoの検索結果ページからタイトルとスニペットを抽出
fn extract_search_results(html: &str) -> String {
    let document = Html::parse_document(html);

    let result_selector = Selector::parse(".result").unwrap();
    let title_selector = Selector::parse(".result__title a, .result__a").unwrap();
    let snippet_selector = Selector::parse(".result__snippet").unwrap();

    let mut results = Vec::new();
    for (i, result) in document.select(&result_selector).enumerate() {
        if i >= 8 {
            break;
        }

        let title = result
            .select(&title_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default()
            .trim()
            .to_string();

        let snippet = result
            .select(&snippet_selector)
            .next()
            .map(|el| el.text().collect::<Vec<_>>().join(""))
            .unwrap_or_default()
            .trim()
            .to_string();

        if !title.is_empty() {
            results.push(format!("{}. {}\n   {}", i + 1, title, snippet));
        }
    }

    results.join("\n\n")
}

/// テキストを文末境界で切り詰める
fn truncate_to_limit(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }

    let truncated: String = text.chars().take(max_chars).collect();

    // 文末境界を探す
    for delim in &['。', '\n', '.', '、'] {
        if let Some(pos) = truncated.rfind(*delim) {
            if pos > max_chars / 2 {
                return truncated[..=pos].to_string();
            }
        }
    }

    truncated
}
