/// Результат поиска: название и ссылка на страницу аниме
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
}

/// Информация о серии: номер и ссылка на видеопоток
#[derive(Debug, Clone)]
pub struct Episode {
    //pub number: String,
    pub video_url: String,
}
