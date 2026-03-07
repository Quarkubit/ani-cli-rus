use crate::models::{Episode, SearchResult};

/// Показывает информацию о тайтле и список серий.
/// Пока что — заглушка
pub fn view(anime: &SearchResult, _cookies: &str) -> Result<Option<Episode>, String> {
    println!("\n══════════════════════════════════════════════════════════════════════════════");
    println!("Title:\t{}", anime.title);
    println!("URL:\t{}", anime.url);
    println!("══════════════════════════════════════════════════════════════════════════════");

    // TODO: Здесь будет загрузка страницы и парсинг серий
    println!("\n    Просмотр серий в разработке...");
    println!("   (Скоро здесь будет список серий и плеер)");
    println!("\n══════════════════════════════════════════════════════════════════════════════");

    // В будущем: вернём Episode с URL видео
    // Сейчас возвращаем None (ничего не воспроизводим)
    Ok(None)
}

/// TODO: функция для выбора конкретной серии
#[allow(dead_code)]
pub fn select_episode(episodes: Vec<Episode>) -> Result<Option<Episode>, String> {
    // TODO: Реализация выбора серии пользователем
    Ok(episodes.into_iter().next())
}
