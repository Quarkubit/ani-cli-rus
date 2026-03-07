// main.rs
mod models;
mod search;
mod title;

use std::io::{self, Write};
// ← Убран неиспользуемый импорт: use models::SearchResult;

fn main() {
    let (user_hash, cookies) = match load_config() {
        Ok(cfg) => cfg,
        Err(e) => {
            eprintln!("❌ Ошибка: {}", e);
            return;
        }
    };

    println!("\n🎬 Jutsu CLI\n");

    loop {
        print!("[1] Поиск  [q] Выход > ");
        io::stdout().flush().unwrap();

        let mut cmd = String::new();
        io::stdin().read_line(&mut cmd).unwrap();

        match cmd.trim() {
            "1" => run_search_flow(&user_hash, &cookies),
            "q" | "Q" => {
                println!("До свидания!");
                break;
            }
            _ => println!("Неизвестная команда\n"),
        }
    }
}

fn load_config() -> Result<(String, String), &'static str> {
    Ok((
        std::env::var("JUTSU_USER_HASH").map_err(|_| "Нет JUTSU_USER_HASH")?,
        std::env::var("JUTSU_COOKIES").map_err(|_| "Нет JUTSU_COOKIES")?,
    ))
}

fn run_search_flow(user_hash: &str, cookies: &str) {
    print!("Поисковый запрос: ");
    io::stdout().flush().unwrap();
    let mut query = String::new();
    io::stdin().read_line(&mut query).unwrap();
    let query = query.trim();

    if query.is_empty() {
        println!("→ Пустой запрос\n");
        return;
    }

    let results = match search::run(query, user_hash, cookies) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("❌ Ошибка поиска: {}\n", e);
            return;
        }
    };

    if results.is_empty() {
        println!("→ Ничего не найдено\n");
        return;
    }

    println!("\n✓ Найдено: {}", results.len());
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
            match title::view(selected, cookies) {
                Ok(Some(episode)) => {
                    play_video(&episode.video_url);
                }
                Ok(None) => {
                    println!("→ Возврат в меню\n");
                }
                Err(e) => eprintln!("❌ Ошибка: {}\n", e),
            }
        } else {
            println!("→ Отменено\n");
        }
    } else {
        println!("→ Некорректный ввод\n");
    }
}

#[allow(dead_code)]
fn play_video(url: &str) {
    println!("▶  Запуск плеера: {}", url);
}
