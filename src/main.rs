use scraper::{Html, Selector};
use std::io::{self, Write};
use std::process::Command;

const BASE_URL: &str = "https://jutsu.tv";

struct SearchResult {
    title: String,
    url: String,
}

fn main() {
    let user_hash = match std::env::var("JUTSU_USER_HASH") {
        Ok(hash) => hash,
        Err(_) => {
            eprintln!("Ошибка: Переменная окружения JUTSU_USER_HASH не установлена.");
            eprintln!("Пример: export JUTSU_USER_HASH='ваш_хеш'");
            return;
        }
    };

    let cookies = match std::env::var("JUTSU_COOKIES") {
        Ok(c) => c,
        Err(_) => {
            eprintln!("Ошибка: Переменная окружения JUTSU_COOKIES не установлена.");
            return;
        }
    };

    print!("Введите поисковый запрос: ");
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();

    if query.is_empty() {
        eprintln!("Запрос не может быть пустым.");
        return;
    }

    match search_anime(query, &user_hash, &cookies) {
        Ok(results) => {
            if results.is_empty() {
                println!("Ничего не найдено.");
                return;
            }

            println!("\n✓ Найдено результатов: {}", results.len());
            for (index, result) in results.iter().enumerate() {
                println!("{}) {} → {}", index + 1, result.title, result.url);
            }

            print!("\nВыберите номер тайтла (или нажмите Enter для выхода): ");
            io::stdout().flush().unwrap();
            let mut choice = String::new();
            io::stdin().read_line(&mut choice).unwrap();

            if let Ok(num) = choice.trim().parse::<usize>() {
                if num > 0 && num <= results.len() {
                    println!("\n✓ Ссылка: {}", results[num - 1].url);
                }
            }
        }
        Err(e) => eprintln!("✗ Ошибка: {}", e),
    }
}

fn search_anime(
    query: &str,
    user_hash: &str,
    cookies: &str,
) -> Result<Vec<SearchResult>, Box<dyn std::error::Error>> {
    // Формируем команду curl как единую строку
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

    let output = Command::new("sh").args(&["-c", &curl_cmd]).output()?;

    let body = String::from_utf8_lossy(&output.stdout).to_string();

    // Проверка на ошибки
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("curl failed: {}", stderr).into());
    }

    if body.contains("File not found") || body.trim().is_empty() {
        return Err("Сервер вернул ошибку или пустой ответ".into());
    }

    // Парсинг HTML
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
                        format!("{}{}", BASE_URL, href_trimmed)
                    };
                    results.push(SearchResult {
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
