#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

#[async_trait::async_trait]
pub trait WebSearcher {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String>;
    async fn fetch_and_extract_text(&self, url: &str) -> Result<String, String>;
}
