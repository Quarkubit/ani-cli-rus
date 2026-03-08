use crate::models::{DownloadedFile, Episode};
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
        .map(|c| if c.is_alphanumeric() || c == '-' || c == '_' || c == ' ' { c } else { '_' })
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

    // Используем curl для скачивания с прогрессом
    let curl_cmd = format!(
        "curl -L '{}' -o '{}' --progress-bar",
        episode.video_url,
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
