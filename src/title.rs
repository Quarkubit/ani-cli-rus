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
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36' --compressed",
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

    // Ищем элементы с data-episode и data-number
    let episode_selector = Selector::parse("[data-episode][data-number]").unwrap();
    
    let mut episodes: Vec<(String, String)> = Vec::new();
    
    for element in document.select(&episode_selector) {
        if let Some(episode_id) = element.value().attr("data-episode") {
            if let Some(episode_num) = element.value().attr("data-number") {
                episodes.push((episode_id.to_string(), episode_num.to_string()));
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
            match get_episode_video_url(&anime_id, episode_id) {
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
fn get_episode_video_url(anime_id: &str, episode_id: &str) -> Result<String, String> {
    // AnimeGO хранит видео на внешних плеерах (kodik, videocdn, etc.)
    // Для получения ссылки нужно сделать запрос к API сайта
    
    // Пробуем получить страницу серии
    let schedule_url = format!(
        "https://animego.me/anime/{}/{}/schedule/load",
        anime_id, episode_id
    );
    
    let curl_cmd = format!(
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36' --compressed -H 'X-Requested-With: XMLHttpRequest'",
        schedule_url
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
