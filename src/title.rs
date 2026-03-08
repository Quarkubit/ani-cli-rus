use std::io::Write;
use crate::models::{Episode, SearchResult};
use scraper::{Html, Selector};

/// Показывает информацию о тайтле и список серий.
pub fn view(anime: &SearchResult) -> Result<Option<Episode>, String> {
    println!("\n═══════════════════════════════════════════════════════════");
    println!("Title:\t{}", anime.title);
    println!("URL:\t{}", anime.url);
    println!("═══════════════════════════════════════════════════════════");

    // Загружаем страницу аниме для получения списка серий
    let curl_cmd = format!(
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8' -H 'Accept-Language: ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7'",
        anime.url
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("Ошибка загрузки страницы: {}", e))?;

    if !output.status.success() {
        return Err("Не удалось загрузить страницу аниме".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    let document = Html::parse_document(&body);

    // Ищем элементы с сериями - пробуем несколько селекторов
    let mut episodes: Vec<(String, String)> = Vec::new();
    
    // Селектор 1: data-episode-id и data-number (новый формат)
    let selector1 = Selector::parse("[data-episode-id][data-number]").unwrap();
    for element in document.select(&selector1) {
        if let Some(episode_id) = element.value().attr("data-episode-id") {
            if let Some(episode_num) = element.value().attr("data-number") {
                episodes.push((episode_id.to_string(), episode_num.to_string()));
            }
        }
    }
    
    // Селектор 2: data-episode и data-number (старый формат)
    if episodes.is_empty() {
        let selector2 = Selector::parse("[data-episode][data-number]").unwrap();
        for element in document.select(&selector2) {
            if let Some(episode_id) = element.value().attr("data-episode") {
                if let Some(episode_num) = element.value().attr("data-number") {
                    episodes.push((episode_id.to_string(), episode_num.to_string()));
                }
            }
        }
    }
    
    // Селектор 3: ищем по классу episode-item
    if episodes.is_empty() {
        let selector3 = Selector::parse(".episode-item[data-id]").unwrap();
        for element in document.select(&selector3) {
            if let Some(episode_id) = element.value().attr("data-id") {
                // Пытаемся найти номер серии в тексте или атрибуте
                let episode_num = element.value().attr("data-number")
                    .map(|s| s.to_string())
                    .or_else(|| {
                        let text: String = element.text().collect::<Vec<_>>().join("");
                        let digits: String = text.trim().chars().filter(|c| c.is_ascii_digit()).collect();
                        if digits.is_empty() {
                            None
                        } else {
                            Some(digits)
                        }
                    })
                    .unwrap_or_else(|| episode_id.to_string());
                episodes.push((episode_id.to_string(), episode_num));
            }
        }
    }
    
    // Селектор 4: ищем все ссылки на серии в блоке серий
    if episodes.is_empty() {
        let selector4 = Selector::parse("a[href*='/anime/'][href*='/']").unwrap();
        for element in document.select(&selector4) {
            if let Some(href) = element.value().attr("href") {
                // Извлекаем ID серии из URL
                let parts: Vec<&str> = href.trim_end_matches('/').split('/').collect();
                if parts.len() >= 3 {
                    if let Some(episode_id) = parts.last() {
                        if episode_id.parse::<i32>().is_ok() {
                            episodes.push((episode_id.to_string(), episode_id.to_string()));
                        }
                    }
                }
            }
        }
    }

    // Если нашли только часть серий, пробуем загрузить все через AJAX
    let anime_id = extract_anime_id(&anime.url).unwrap_or_default();
    if !anime_id.is_empty() && !episodes.is_empty() {
        // Берем последний episode_id для запроса
        if let Some((last_episode_id, _)) = episodes.last() {
            let schedule_url = format!(
                "https://animego.me/anime/{}/{}/schedule/load",
                anime_id, last_episode_id
            );
            
            let curl_cmd = format!(
                "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'X-Requested-With: XMLHttpRequest' -H 'Accept: application/json, text/javascript, */*; q=0.01'",
                schedule_url
            );
            
            if let Ok(output) = std::process::Command::new("sh")
                .arg("-c")
                .arg(&curl_cmd)
                .output() 
            {
                let body = String::from_utf8_lossy(&output.stdout).to_string();
                // Парсим JSON ответ и извлекаем все серии
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&body) {
                    if let Some(data) = json.get("data").and_then(|d| d.get("content")) {
                        if let Some(content) = data.as_str() {
                            let doc = Html::parse_document(content);
                            let selector = Selector::parse("[data-episode][data-number]").unwrap();
                            
                            let mut all_episodes: Vec<(String, String)> = Vec::new();
                            for element in doc.select(&selector) {
                                if let Some(ep_id) = element.value().attr("data-episode") {
                                    if let Some(ep_num) = element.value().attr("data-number") {
                                        all_episodes.push((ep_id.to_string(), ep_num.to_string()));
                                    }
                                }
                            }
                            
                            if !all_episodes.is_empty() {
                                episodes = all_episodes;
                            }
                        }
                    }
                }
            }
        }
    }

    // Удаляем дубликаты и сортируем по номеру серии
    episodes.sort_by(|a, b| {
        let num_a: i32 = a.1.parse().unwrap_or(0);
        let num_b: i32 = b.1.parse().unwrap_or(0);
        num_a.cmp(&num_b)
    });
    episodes.dedup();

    if episodes.is_empty() {
        println!("\n    Серии не найдены или требуют авторизации...");
        println!("   (Возможно, аниме еще не вышло или недоступно)");
        println!("\n═══════════════════════════════════════════════════════════");
        return Ok(None);
    }

    println!("\nНайдено серий: {}", episodes.len());
    
    // Показываем последние 20 серий
    let display_count = std::cmp::min(20, episodes.len());
    let start_idx = episodes.len() - display_count;
    
    println!("\nПоследние {} серий:", display_count);
    for (_, (_, num)) in episodes.iter().enumerate().skip(start_idx) {
        println!("  Серия {}", num);
    }

    print!("\nВыберите серию для просмотра (0 - отмена): ");
    std::io::stdout().flush().unwrap();
    
    let mut choice = String::new();
    std::io::stdin().read_line(&mut choice).unwrap();

    if let Ok(num) = choice.trim().parse::<usize>() {
        if num > 0 && num <= episodes.len() {
            let (episode_id, episode_num) = &episodes[num - 1];
            
            // Извлекаем anime_id из URL
            let anime_id = extract_anime_id(&anime.url)?;
            
            println!("\nЗагрузка информации о серии {}...", episode_num);
            
            // Пытаемся получить информацию о плеере
            match get_episode_video_url(&anime.url, episode_id) {
                Ok(video_url) => {
                    println!("✓ Видео найдено!");
                    return Ok(Some(Episode {
                        number: episode_num.clone(),
                        video_url,
                        anime_title: anime.title.clone(),
                    }));
                }
                Err(e) => {
                    eprintln!("⚠ Не удалось получить ссылку на видео: {}", e);
                    eprintln!("  Но вы можете попробовать скачать серию напрямую");
                    
                    // Возвращаем заглушку для возможности скачивания
                    return Ok(Some(Episode {
                        number: episode_num.clone(),
                        video_url: format!("https://animego.me/anime/{}/{}", anime_id, episode_id),
                        anime_title: anime.title.clone(),
                    }));
                }
            }
        } else {
            println!("Отменено\n");
        }
    } else {
        println!("Некорректный ввод\n");
    }

    println!("\n═══════════════════════════════════════════════════════════");
    Ok(None)
}

/// Извлекает ID аниме из URL
fn extract_anime_id(url: &str) -> Result<String, String> {
    // URL вида: https://animego.me/anime/naruto-uragannye-hroniki-103
    // ID это последняя часть после последнего дефиса, если это число
    let parts: Vec<&str> = url.trim_end_matches('/').split('/').collect();
    if let Some(slug) = parts.last() {
        // Пытаемся найти число в конце slug
        let id: String = slug.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() {
            return Ok(id.chars().rev().collect());
        }
    }
    Err("Не удалось извлечь ID аниме из URL".to_string())
}

/// Получает URL видео для серии
fn get_episode_video_url(anime_url: &str, episode_id: &str) -> Result<String, String> {
    // AnimeGO хранит видео на внешних плеерах (kodik, videocdn, etc.)
    // Для получения ссылки нужно загрузить страницу серии и найти iframe
    
    let curl_cmd = format!(
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8' -H 'Accept-Language: ru-RU,ru;q=0.9,en-US;q=0.8,en;q=0.7'",
        anime_url
    );
    
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("Ошибка запроса: {}", e))?;
    
    let body = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Ищем iframe или source с видео
    if let Some(video_url) = extract_video_url_from_html(&body) {
        return Ok(video_url);
    }
    
    // Пробуем получить страницу через API schedule/load
    let parts: Vec<&str> = anime_url.trim_end_matches('/').split('/').collect();
    if let Some(slug) = parts.last() {
        let id: String = slug.chars().rev().take_while(|c| c.is_ascii_digit()).collect();
        if !id.is_empty() {
            let anime_id = id.chars().rev().collect::<String>();
            let schedule_url = format!(
                "https://animego.me/anime/{}/{}/schedule/load",
                anime_id, episode_id
            );
            
            let curl_cmd = format!(
                "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'X-Requested-With: XMLHttpRequest' -H 'Accept: application/json, text/javascript, */*; q=0.01'",
                schedule_url
            );
            
            if let Ok(output) = std::process::Command::new("sh")
                .arg("-c")
                .arg(&curl_cmd)
                .output() {
                let body = String::from_utf8_lossy(&output.stdout).to_string();
                if let Some(video_url) = extract_video_url_from_html(&body) {
                    return Ok(video_url);
                }
            }
        }
    }
    
    Err("Видео не найдено".to_string())
}

/// Извлекает URL видео из HTML
fn extract_video_url_from_html(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    
    // Ищем iframe
    if let Ok(iframe_selector) = Selector::parse("iframe") {
        if let Some(iframe) = document.select(&iframe_selector).next() {
            if let Some(src) = iframe.value().attr("src") {
                if src.contains("kodik") || src.contains("videocdn") || src.contains("alloha") {
                    return Some(src.to_string());
                }
            }
        }
    }
    
    // Ищем source
    if let Ok(source_selector) = Selector::parse("source") {
        if let Some(source) = document.select(&source_selector).next() {
            if let Some(src) = source.value().attr("src") {
                return Some(src.to_string());
            }
        }
    }
    
    // Ищем data-player-src или подобные атрибуты
    if let Some(start) = html.find("data-player-src=\"") {
        let rest = &html[start + 17..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    
    None
}

/// Функция для выбора конкретной серии
#[allow(dead_code)]
pub fn select_episode(episodes: Vec<Episode>) -> Result<Option<Episode>, String> {
    // TODO: Реализация выбора серии пользователем
    Ok(episodes.into_iter().next())
}
