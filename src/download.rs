use crate::models::{DownloadedFile, Episode};
use scraper::{Html, Selector};
use std::fs;
use std::io::{self, Write};
use std::path::Path;

const DOWNLOAD_DIR: &str = "/tmp/ani-cli-rus";

/// Инициализация директории для скачанных файлов
pub fn init_download_dir() -> Result<(), String> {
    if !Path::new(DOWNLOAD_DIR).exists() {
        fs::create_dir_all(DOWNLOAD_DIR)
            .map_err(|e| format!("Не удалось создать директорию {}: {}", DOWNLOAD_DIR, e))?;
    }
    Ok(())
}

/// Генерация безопасного имени файла из названия аниме и номера серии
fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' || c.is_ascii() { c } else { '_' })
        .collect::<String>()
        .trim()
        .to_string()
}

/// Скачивание видео по URL в локальный файл
pub fn download_episode(episode: &Episode) -> Result<DownloadedFile, String> {
    init_download_dir()?;

    let safe_title = sanitize_filename(&episode.anime_title);
    let safe_ep = sanitize_filename(&episode.number);
    let filename = format!("{}_{}.mp4", safe_title, safe_ep);
    let file_path = format!("{}/{}", DOWNLOAD_DIR, filename);

    println!("Скачивание: {} (серия {})", episode.anime_title, episode.number);
    println!("URL: {}", episode.video_url);
    println!("Путь сохранения: {}", file_path);

    // Проверяем, является ли URL прямой ссылкой на видео или страницей animego
    let download_url = if episode.video_url.contains("animego.me") {
        // Это страница animego, нужно извлечь реальную ссылку на видео
        extract_direct_video_url(&episode.video_url)?
    } else {
        episode.video_url.clone()
    };

    // Используем curl для скачивания с прогрессом
    let curl_cmd = format!(
        "curl -L '{}' -o '{}' --progress-bar",
        download_url,
        file_path
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .status()
        .map_err(|e| format!("Ошибка запуска curl: {}", e))?;

    if !output.success() {
        return Err("Скачивание не удалось".to_string());
    }

    // Проверка, что файл существует и не пустой
    if !Path::new(&file_path).exists() {
        return Err("Файл не был создан после скачивания".to_string());
    }

    let metadata = fs::metadata(&file_path)
        .map_err(|e| format!("Не удалось получить метаданные файла: {}", e))?;

    if metadata.len() == 0 {
        fs::remove_file(&file_path).ok();
        return Err("Скачанный файл пустой".to_string());
    }

    println!("✓ Скачано успешно! ({:.2} MB)", metadata.len() as f64 / 1024.0 / 1024.0);

    Ok(DownloadedFile {
        file_path,
        anime_title: episode.anime_title.clone(),
        episode_number: episode.number.clone(),
    })
}

/// Извлекает прямую ссылку на видео со страницы animego
fn extract_direct_video_url(page_url: &str) -> Result<String, String> {
    let curl_cmd = format!(
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8'",
        page_url
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("Ошибка загрузки страницы: {}", e))?;

    if !output.status.success() {
        return Err("Не удалось загрузить страницу для получения видео".to_string());
    }

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Ищем iframe с плеером
    if let Some(iframe_src) = extract_iframe_src(&body) {
        // Загружаем страницу iframe для получения прямой ссылки
        return extract_video_from_player(&iframe_src);
    }

    // Ищем direct source
    if let Some(source_url) = extract_source_url(&body) {
        return Ok(source_url);
    }

    Err("Не удалось найти прямую ссылку на видео".to_string())
}

/// Извлекает src из iframe
fn extract_iframe_src(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    if let Ok(selector) = Selector::parse("iframe") {
        if let Some(iframe) = document.select(&selector).next() {
            if let Some(src) = iframe.value().attr("src") {
                return Some(src.to_string());
            }
        }
    }
    
    // Ищем в атрибутах data-player-src и подобных
    if let Some(start) = html.find("data-player-src=\"") {
        let rest = &html[start + 17..];
        if let Some(end) = rest.find('"') {
            return Some(rest[..end].to_string());
        }
    }
    
    None
}

/// Извлекает video URL из player страницы
fn extract_video_from_player(player_url: &str) -> Result<String, String> {
    let curl_cmd = format!(
        "curl -sL '{}' -H 'User-Agent: Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/120.0.0.0 Safari/537.36' --compressed -H 'Accept: text/html,application/xhtml+xml,application/xml;q=0.9,image/webp,*/*;q=0.8' -H 'Referer: https://animego.me/'",
        player_url
    );

    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(&curl_cmd)
        .output()
        .map_err(|e| format!("Ошибка загрузки плеера: {}", e))?;

    let body = String::from_utf8_lossy(&output.stdout).to_string();
    
    // Ищем m3u8 или mp4 ссылки
    if let Some(video_url) = extract_source_url(&body) {
        return Ok(video_url);
    }
    
    // Ищем JSON с источниками
    if let Some(json_start) = body.find("\"sources\"") {
        if let Some(json_end) = body[json_start..].find("}]") {
            let json_part = &body[json_start..json_start + json_end + 2];
            if let Ok(json) = serde_json::from_str::<serde_json::Value>(json_part) {
                if let Some(sources) = json.get("sources").and_then(|s| s.as_array()) {
                    for source in sources {
                        if let Some(url) = source.get("file").and_then(|f| f.as_str()) {
                            return Ok(url.to_string());
                        }
                    }
                }
            }
        }
    }

    Err("Не удалось извлечь видео из плеера".to_string())
}

/// Извлекает URL видео из HTML (source tag или类似)
fn extract_source_url(html: &str) -> Option<String> {
    let document = Html::parse_document(html);
    
    // Ищем source tag
    if let Ok(selector) = Selector::parse("source") {
        if let Some(source) = document.select(&selector).next() {
            if let Some(src) = source.value().attr("src") {
                return Some(src.to_string());
            }
        }
    }
    
    // Ищем video tag с src
    if let Ok(selector) = Selector::parse("video") {
        if let Some(video) = document.select(&selector).next() {
            if let Some(src) = video.value().attr("src") {
                return Some(src.to_string());
            }
        }
    }
    
    None
}

/// Отображение списка скачанных файлов
pub fn list_downloaded() -> Result<Vec<DownloadedFile>, String> {
    init_download_dir()?;

    let mut files = Vec::new();

    let entries = fs::read_dir(DOWNLOAD_DIR)
        .map_err(|e| format!("Не удалось прочитать директорию {}: {}", DOWNLOAD_DIR, e))?;

    for entry in entries {
        let entry = entry.map_err(|e| format!("Ошибка чтения entry: {}", e))?;
        let path = entry.path();

        if path.is_file() {
            if let Some(ext) = path.extension() {
                if ext == "mp4" || ext == "mkv" || ext == "avi" {
                    let filename = path
                        .file_name()
                        .and_then(|n| n.to_str())
                        .unwrap_or("unknown")
                        .to_string();

                    // Пытаемся извлечь название аниме и номер серии из имени файла
                    // Формат: Title_Ep.mp4
                    let parts: Vec<&str> = filename.trim_end_matches(".mp4").trim_end_matches(".mkv").trim_end_matches(".avi").split('_').collect();
                    
                    let (anime_title, episode_number) = if parts.len() >= 2 {
                        (parts[..parts.len()-1].join("_"), parts[parts.len()-1].to_string())
                    } else {
                        (filename.clone(), "unknown".to_string())
                    };

                    files.push(DownloadedFile {
                        file_path: path.to_string_lossy().to_string(),
                        anime_title,
                        episode_number,
                    });
                }
            }
        }
    }

    files.sort_by(|a, b| a.anime_title.cmp(&b.anime_title));

    Ok(files)
}

/// Выбор скачанного файла для просмотра
pub fn select_downloaded() -> Result<Option<DownloadedFile>, String> {
    let files = list_downloaded()?;

    if files.is_empty() {
        println!("\nНет скачанных файлов.");
        return Ok(None);
    }

    println!("\n=== Скачанные файлы ===");
    for (i, file) in files.iter().enumerate() {
        println!("{}) {} (Серия {})", i + 1, file.anime_title, file.episode_number);
    }

    print!("\nВыберите файл (0 - отмена): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    if let Ok(num) = choice.trim().parse::<usize>() {
        if num > 0 && num <= files.len() {
            return Ok(Some(files[num - 1].clone()));
        }
    }

    Ok(None)
}

/// Воспроизведение локального файла
pub fn play_local_file(file_path: &str) {
    println!("\nВоспроизведение: {}", file_path);
    
    // Попытка воспроизвести через mpv, vlc или системный плеер
    let players = ["mpv", "vlc", "xdg-open", "open"];
    
    for player in &players {
        let status = std::process::Command::new(player)
            .arg(file_path)
            .status();
        
        if status.is_ok() {
            return;
        }
    }
    
    println!("Не удалось запустить видеоплеер. Установите mpv или vlc.");
}

/// Удаление одного скачанного файла
pub fn delete_single_file() -> Result<(), String> {
    let files = list_downloaded()?;

    if files.is_empty() {
        println!("\nНет скачанных файлов для удаления.");
        return Ok(());
    }

    println!("\n=== Скачанные файлы ===");
    for (i, file) in files.iter().enumerate() {
        println!("{}) {} (Серия {})", i + 1, file.anime_title, file.episode_number);
    }

    print!("\nВыберите файл для удаления (0 - отмена): ");
    io::stdout().flush().unwrap();

    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    if let Ok(num) = choice.trim().parse::<usize>() {
        if num > 0 && num <= files.len() {
            let file = &files[num - 1];
            println!("\nВы уверены, что хотите удалить:");
            println!("  {} (Серия {})", file.anime_title, file.episode_number);
            print!("Подтвердить удаление? (y/n): ");
            io::stdout().flush().unwrap();
            
            let mut confirm = String::new();
            io::stdin().read_line(&mut confirm).unwrap();
            
            if confirm.trim().to_lowercase() == "y" {
                fs::remove_file(&file.file_path)
                    .map_err(|e| format!("Ошибка удаления файла: {}", e))?;
                println!("✓ Файл успешно удален!");
            } else {
                println!("Удаление отменено.");
            }
        }
    }

    Ok(())
}

/// Удаление всех скачанных файлов
pub fn delete_all_files() -> Result<(), String> {
    let files = list_downloaded()?;

    if files.is_empty() {
        println!("\nНет скачанных файлов для удаления.");
        return Ok(());
    }

    println!("\n=== Скачанные файлы ({}) ===", files.len());
    for (i, file) in files.iter().enumerate() {
        println!("{}) {} (Серия {})", i + 1, file.anime_title, file.episode_number);
    }

    println!("\n⚠ ВНИМАНИЕ: Будут удалены ВСЕ скачанные файлы ({})", files.len());
    print!("Введите 'DELETE' для подтверждения: ");
    io::stdout().flush().unwrap();
    
    let mut confirm = String::new();
    io::stdin().read_line(&mut confirm).unwrap();
    
    if confirm.trim() != "DELETE" {
        println!("Удаление отменено.");
        return Ok(());
    }

    print!("Вы действительно уверены? Введите 'YES' для полного удаления: ");
    io::stdout().flush().unwrap();
    
    let mut final_confirm = String::new();
    io::stdin().read_line(&mut final_confirm).unwrap();
    
    if final_confirm.trim() != "YES" {
        println!("Удаление отменено.");
        return Ok(());
    }

    let mut deleted_count = 0;
    let mut errors = Vec::new();
    
    for file in &files {
        match fs::remove_file(&file.file_path) {
            Ok(_) => deleted_count += 1,
            Err(e) => errors.push(format!("{}: {}", file.file_path, e)),
        }
    }

    println!("\n✓ Удалено файлов: {}", deleted_count);
    
    if !errors.is_empty() {
        println!("Ошибки при удалении:");
        for err in errors {
            println!("  - {}", err);
        }
    }

    Ok(())
}

/// Меню управления скачанными файлами
pub fn manage_downloaded() -> Result<Option<DownloadedFile>, String> {
    let files = list_downloaded()?;

    if files.is_empty() {
        println!("\nНет скачанных файлов.");
        return Ok(None);
    }

    println!("\n=== Скачанные файлы ===");
    for (i, file) in files.iter().enumerate() {
        println!("{}) {} (Серия {})", i + 1, file.anime_title, file.episode_number);
    }

    println!("\nДоступные действия:");
    println!("1. Воспроизвести файл");
    println!("2. Удалить отдельный файл");
    println!("3. Удалить ВСЁ скачанное");
    println!("0. Отмена");
    
    print!("\nВыбор: ");
    io::stdout().flush().unwrap();

    let mut action = String::new();
    io::stdin().read_line(&mut action).unwrap();

    match action.trim() {
        "1" => {
            print!("Выберите файл для воспроизведения (0 - отмена): ");
            io::stdout().flush().unwrap();
            
            let mut choice = String::new();
            io::stdin().read_line(&mut choice).unwrap();
            
            if let Ok(num) = choice.trim().parse::<usize>() {
                if num > 0 && num <= files.len() {
                    return Ok(Some(files[num - 1].clone()));
                }
            }
            Ok(None)
        }
        "2" => {
            delete_single_file()?;
            Ok(None)
        }
        "3" => {
            delete_all_files()?;
            Ok(None)
        }
        _ => Ok(None),
    }
}
