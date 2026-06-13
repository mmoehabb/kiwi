/// The Web component acts as a bridge for the LLM when it lacks up-to-date information.
/// It provides capabilities for searching Google and scraping web pages for context.

#[async_trait::async_trait]
pub trait WebSearcher {
    /// Queries a search engine and returns a list of result snippets and URLs.
    /// TODO: Implement a Google Search API integration or a custom scraper tool.
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String>;

    /// Fetches the raw HTML of a given URL and extracts the readable text.
    /// Useful for feeding articles or documentation into the LLM context.
    /// TODO: Implement HTTP client (e.g., reqwest) and HTML parsing (e.g., scraper).
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
use reqwest::Client;
use scraper::{Html, Selector};
use std::sync::Arc;

/// The main struct handling outgoing web requests.
pub struct WebClient {
    client: Client,
    config: Arc<Configuration>,
}

impl WebClient {
    pub fn new(config: Arc<Configuration>) -> Self {
        let client = Client::builder()
            .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/91.0.4472.124 Safari/537.36")
            .build()
            .unwrap_or_default();
        Self { client, config }
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

        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let html_content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let document = Html::parse_document(&html_content);
        let result_selector =
            Selector::parse(".result").map_err(|_| "Invalid selector".to_string())?;
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

    async fn fetch_and_extract_text(&self, url: &str) -> Result<String, String> {
        let response = self
            .client
            .get(url)
            .send()
            .await
            .map_err(|e| format!("Failed to send request: {}", e))?;

        let html_content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response: {}", e))?;

        let document = Html::parse_document(&html_content);
        let body_selector = Selector::parse("body").map_err(|_| "Invalid selector".to_string())?;
        let remove_selectors = ["script", "style", "noscript", "iframe", "svg"];

        if let Some(body) = document.select(&body_selector).next() {
            let mut extracted_text = String::new();
            for node in body.descendants() {
                if let Some(element) = node.value().as_element() {
                    let tag_name = element.name();
                    if remove_selectors.contains(&tag_name) {
                        continue;
                    }
                }
                if let Some(text_node) = node.value().as_text() {
                    let mut should_skip = false;
                    for ancestor in node.ancestors() {
                        if let Some(el) = ancestor.value().as_element()
                            && remove_selectors.contains(&el.name())
                        {
                            should_skip = true;
                            break;
                        }
                    }
                    if !should_skip {
                        extracted_text.push_str(&text_node.text);
                        extracted_text.push(' ');
                    }
                }
            }

            // basic whitespace compression
            let compressed_text: String = extracted_text
                .split_whitespace()
                .collect::<Vec<_>>()
                .join(" ");
            Ok(compressed_text)
        } else {
            Err("No body element found in HTML".to_string())
        }
    }
}
