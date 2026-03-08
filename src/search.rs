use crate::models::SearchResult;
use scraper::{Html, Selector};

const BASE_URL: &str = "https://animego.me";

pub fn run(query: &str) -> Result<Vec<SearchResult>, String> {
    let curl_cmd = format!(
        "curl -sL 'https://animego.me/search/all?q={}' \
         -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36' \
         --compressed",
        urlencoding::encode(query)
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("System error: {}", e))?;

    if !output.status.success() {
        return Err("Search request failed".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    if body.contains("404") || body.trim().is_empty() {
        return Err("No results found".to_string());
    }

    let document = Html::parse_document(&body);
    // Селектор для ссылок на аниме (теперь это просто <a> с href="/anime/..." и title)
    let link_selector = Selector::parse("a[href^='/anime/'][title]").unwrap();

    let mut results = Vec::new();

    for element in document.select(&link_selector) {
        if let Some(href) = element.value().attr("href") {
            let href_trimmed = href.trim().to_string();
            
            // Пропускаем служебные ссылки (онгоинги, сезоны и т.д.)
            if href_trimmed.starts_with("/anime/status") 
                || href_trimmed.starts_with("/anime/season")
                || href_trimmed.starts_with("/anime/random")
            {
                continue;
            }

            // Получаем заголовок из атрибута title
            let title = if let Some(title_attr) = element.value().attr("title") {
                title_attr.trim().to_string()
            } else {
                element.text().collect::<String>().trim().to_string()
            };

            if !title.is_empty() && href_trimmed.starts_with("/anime/") {
                let full_url = if href_trimmed.starts_with("http") {
                    href_trimmed
                } else {
                    format!("{}{}", BASE_URL, href_trimmed)
                };

                results.push(SearchResult {
                    title,
                    url: full_url,
                });
            }
        }
    }

    results.dedup_by(|a, b| a.url == b.url);
    Ok(results)
}

