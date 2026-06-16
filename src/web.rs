/// The Web component acts as a bridge for the LLM when it lacks up-to-date information.
/// It provides capabilities for searching Google and scraping web pages for context.

#[async_trait::async_trait]
pub trait WebSearcher {
    /// Queries a search engine and returns a list of result snippets and URLs.
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String>;

    /// Fetches the raw HTML of a given URL and extracts the readable text.
    /// Useful for feeding articles or documentation into the LLM context.
    async fn fetch_and_extract_text(&self, url: &str) -> Result<String, String>;
}

/// A structured result from a web search.
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub title: String,
    pub url: String,
    pub snippet: String,
}

use crate::config::Configuration;
use crate::llm::LlmEngine;
use scraper::{Html, Selector};
use std::io::Cursor;
use std::sync::Arc;

/// The main struct handling outgoing web requests via w3m.
pub struct WebClient {
    config: Arc<Configuration>,
}

impl WebClient {
    pub fn new(config: Arc<Configuration>) -> Self {
        Self { config }
    }
}

#[async_trait::async_trait]
impl WebSearcher for WebClient {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let url = self
            .config
            .app
            .search_url_template
            .replace("{}", &urlencoding::encode(query));

        let output = tokio::process::Command::new("w3m")
            .arg("-dump_source")
            .arg(&url)
            .output()
            .await
            .map_err(|e| format!("Failed to run w3m: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("w3m exited with error: {}", stderr));
        }

        let html = decompress_brotli(&output.stdout)?;
        parse_search_results(&html)
    }

    async fn fetch_and_extract_text(&self, url: &str) -> Result<String, String> {
        let output = tokio::process::Command::new("w3m")
            .arg("-dump")
            .arg(url)
            .output()
            .await
            .map_err(|e| format!("Failed to run w3m: {}", e))?;

        if !output.status.success() {
            let stderr = String::from_utf8_lossy(&output.stderr);
            return Err(format!("w3m exited with error: {}", stderr));
        }

        let text = String::from_utf8_lossy(&output.stdout).to_string();
        Ok(text.split_whitespace().collect::<Vec<_>>().join(" "))
    }
}

fn decompress_brotli(data: &[u8]) -> Result<String, String> {
    let mut reader = Cursor::new(data);
    let mut decompressed = Vec::new();
    match brotli::BrotliDecompress(&mut reader, &mut decompressed) {
        Ok(_) => String::from_utf8(decompressed)
            .map_err(|e| format!("Brotli output is not valid UTF-8: {}", e)),
        Err(_) => {
            // Not brotli-compressed, use raw data
            String::from_utf8(data.to_vec())
                .map_err(|e| format!("w3m output is not valid UTF-8: {}", e))
        }
    }
}

fn parse_search_results(html: &str) -> Result<Vec<SearchResult>, String> {
    let document = Html::parse_document(html);
    let result_selector = Selector::parse(".result").map_err(|_| "Invalid selector".to_string())?;
    let title_selector =
        Selector::parse(".result__a").map_err(|_| "Invalid selector".to_string())?;
    let snippet_selector =
        Selector::parse(".result__snippet").map_err(|_| "Invalid selector".to_string())?;

    let mut results = Vec::new();

    for element in document.select(&result_selector) {
        let title_el = element.select(&title_selector).next();
        let snippet_el = element.select(&snippet_selector).next();

        if let (Some(t), Some(s)) = (title_el, snippet_el) {
            let title = t.text().collect::<Vec<_>>().join("");
            let url = t.value().attr("href").unwrap_or("").to_string();
            let snippet = s.text().collect::<Vec<_>>().join("");

            results.push(SearchResult {
                title,
                url,
                snippet,
            });
        }
    }

    Ok(results)
}

pub struct WebTool {
    searcher: Arc<dyn WebSearcher + Send + Sync>,
    llm: Arc<dyn LlmEngine + Send + Sync>,
}

impl WebTool {
    pub fn new(
        searcher: Arc<dyn WebSearcher + Send + Sync>,
        llm: Arc<dyn LlmEngine + Send + Sync>,
    ) -> Self {
        Self { searcher, llm }
    }

    pub async fn search_and_recap(&self, query: &str) -> Result<String, String> {
        let results = self.searcher.search(query).await?;

        if results.is_empty() {
            return Err(format!("No search results found for query: {}", query));
        }

        let first_result = &results[0];

        // Try to fetch the full content of the first result's URL
        let mut url_to_fetch = first_result.url.clone();
        if url_to_fetch.starts_with("//") {
            url_to_fetch = format!("https:{}", url_to_fetch);
        }

        let extracted_text = match self.searcher.fetch_and_extract_text(&url_to_fetch).await {
            Ok(text) if !text.trim().is_empty() => text,
            _ => first_result.snippet.clone(),
        };

        // Truncate text to avoid exceeding context window (simple approach)
        let max_chars = 8000;
        let truncated_text: String = extracted_text.chars().take(max_chars).collect();

        let prompt = format!(
            "Based on the following extracted text from a web search, answer the query: '{}'.\n\nSource Title: {}\nSource URL: {}\n\nText:\n{}\n\nRecap:",
            query, first_result.title, url_to_fetch, truncated_text
        );

        let recap = self.llm.generate(&prompt).await?;
        Ok(recap)
    }
}
