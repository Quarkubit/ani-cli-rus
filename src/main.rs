mod models;
mod search;
mod title;
mod download;

use std::io::{self, Write};

fn main() {
    let (user_hash, cookies) = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("Ошибка: {}", e);
            return;
        }
    };

    println!("\n======== AnimeGO CLI ========\n");

    // Инициализация директории для скачанных файлов
    if let Err(e) = download::init_download_dir() {
        eprintln!("Предупреждение: {}", e);
    }

    loop {
        print!("[1] Поиск  [2] Скачанное  [q] Выход > ");
        io::stdout().flush().unwrap();

        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd).unwrap();

        match cmd.trim() {
            "1" => run_search_flow(&user_hash, &cookies),
            "2" => run_downloaded_flow(),
            "q" | "Q" => {
                println!("До свидания!");
                break;
            }
            _ => println!("Неизвестная команда\n"),
        }
    }
}

fn load_config() -> Result<(String, String), &'static str> {
    // Для AnimeGO пока не требуются авторизационные данные для базового поиска
    // Но оставляем возможность расширения
    Ok((
        std::env::var("ANIMEGO_USER_HASH").unwrap_or_default(),
        std::env::var("ANIMEGO_COOKIES").unwrap_or_default(),
    ))
}

fn run_search_flow(_user_hash: &str, _cookies: &str) {
    print!("Поисковый запрос: ");
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();

    if query.is_empty() {
        println!("Пустой запрос!\n");
        return;
    }

    let results = match search::run(query) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Ошибка поиска: {}\n", e);
            return;
        }
    };

    if results.is_empty() {
        println!("Ничего не найдено! Увы :(\n");
        return;
    }

    println!("\nОтлично! Найдено: {}", results.len());
    for (i, item) in results.iter().enumerate() {
        println!("{}) {}", i + 1, item.title);
    }

    print!("\nВыберите тайтл (0 - отмена): ");
    io::stdout().flush().unwrap();
    let mut choice = String::new();
    io::stdin().read_line(&mut choice).unwrap();

    if let Ok(num) = choice.trim().parse::<usize>() {
        if num > 0 && num <= results.len() {
            let selected = &results[num - 1];
            match title::view(selected) {
                Ok(Some(episodes)) => {
                    // episodes теперь Vec<Episode>
                    if episodes.len() == 1 {
                        let episode = &episodes[0];
                        println!("\nДоступные действия:");
                        println!("1. Смотреть онлайн");
                        println!("2. Скачать");
                        println!("0. Отмена");
                        print!("\nВыбор: ");
                        io::stdout().flush().unwrap();
                        
                        let mut action = String::new();
                        io::stdin().read_line(&mut action).unwrap();
                        
                        match action.trim() {
                            "1" => {
                                play_video(&episode.video_url);
                            }
                            "2" => {
                                match download::download_episode(episode) {
                                    Ok(file) => {
                                        println!("✓ Файл сохранен: {}", file.file_path);
                                        print!("Воспроизвести сейчас? (y/n): ");
                                        io::stdout().flush().unwrap();
                                        let mut play_now = String::new();
                                        io::stdin().read_line(&mut play_now).unwrap();
                                        if play_now.trim().to_lowercase() == "y" {
                                            download::play_local_file(&file.file_path);
                                        }
                                    }
                                    Err(e) => {
                                        eprintln!("Ошибка скачивания: {}\n", e);
                                    }
                                }
                            }
                            _ => {
                                println!("Отменено\n");
                            }
                        }
                    }
                    // Если серий несколько, они уже обработаны в title::view
                }
                Ok(None) => {
                    println!("Возврат в меню\n");
                }
                Err(e) => eprintln!("Ошибка: {}\n", e),
            }
        } else {
            println!("Отменено\n");
        }
    } else {
        println!("Некорректный ввод\n");
    }
}

#[allow(dead_code)]
fn play_video(url: &str) {
    println!("Запуск плеера: {}", url);
}

fn run_downloaded_flow() {
    match download::manage_downloaded() {
        Ok(Some(file)) => {
            download::play_local_file(&file.file_path);
        }
        Ok(None) => {
            // Отмена или нет файлов
        }
        Err(e) => {
            eprintln!("Ошибка: {}\n", e);
        }
    }
}
