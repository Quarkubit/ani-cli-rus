use crate::models::SearchResult;
use scraper::{Html, Selector};
use std::process::Command; // ← Исправлено: было AnimeTitle

pub fn run(query: &str, user_hash: &str, cookies: &str) -> Result<Vec<SearchResult>, String> {
    let curl_cmd = format!(
        "curl 'https://jutsu.tv/engine/ajax/controller.php?mod=search' \
         --compressed -X POST \
         -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64; rv:148.0) Gecko/20100101 Firefox/148.0' \
         -H 'Accept: */*' \
         -H 'Content-Type: application/x-www-form-urlencoded; charset=UTF-8' \
         -H 'X-Requested-With: XMLHttpRequest' \
         -H 'Referer: https://jutsu.tv/' \
         -H 'Cookie: {}' \
         -d 'query={}&skin=jutsutv&user_hash={}' \
         -s",
        cookies, query, user_hash
    );

    let output = Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("System error: {}", e))?;

    if !output.status.success() {
        return Err("Search request failed".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.contains("File not found") || body.trim().is_empty() {
        return Err("No results found".to_string());
    }

    let document = Html::parse_document(&body);
    let row_selector = Selector::parse("a.fs-result").unwrap();
    let title_selector = Selector::parse("div.fs-result__title").unwrap();

    let mut results = Vec::new();

    for element in document.select(&row_selector) {
        if let Some(href) = element.value().attr("href") {
            let href_trimmed = href.trim().to_string();
            if let Some(title_elem) = element.select(&title_selector).next() {
                let title = title_elem.text().collect::<String>().trim().to_string();

                if !title.is_empty() && href_trimmed.contains(".html") {
                    let full_url = if href_trimmed.starts_with("http") {
                        href_trimmed
                    } else {
                        format!("https://jutsu.tv{}", href_trimmed)
                    };

                    results.push(SearchResult {
                        // ← Исправлено: было AnimeTitle
                        title,
                        url: full_url,
                    });
                }
            }
        }
    }

    results.dedup_by(|a, b| a.url == b.url);
    Ok(results)
}

