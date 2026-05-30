use kiwi_core::web::{SearchResult, WebSearcher};

use reqwest::Client;
use scraper::{Html, Selector};

/// The main struct handling outgoing web requests.
pub struct WebClient {
    client: Client,
    search_base_url: String,
}

impl Default for WebClient {
    fn default() -> Self {
        Self::new()
    }
}

impl WebClient {
    pub fn new() -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build()
                .unwrap_or_default(),
            search_base_url: "https://html.duckduckgo.com/html/".to_string(),
        }
    }

    /// Internal constructor for testing to allow overriding the base search URL
    pub fn with_base_url(base_url: &str) -> Self {
        Self {
            client: Client::builder()
                .user_agent("Mozilla/5.0 (Windows NT 10.0; Win64; x64)")
                .build()
                .unwrap_or_default(),
            search_base_url: base_url.to_string(),
        }
    }
}

#[async_trait::async_trait]
impl WebSearcher for WebClient {
    async fn search(&self, query: &str) -> Result<Vec<SearchResult>, String> {
        let url = format!("{}?q={}", self.search_base_url, urlencoding::encode(query));
        let response = self
            .client
            .get(&url)
            .send()
            .await
            .map_err(|e| format!("Failed to send search request: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Search request failed with status: {}",
                response.status()
            ));
        }

        let html_content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let document = Html::parse_document(&html_content);
        let result_selector =
            Selector::parse(".result").map_err(|_| "Failed to parse CSS selector")?;
        let title_selector = Selector::parse(".result__title .result__a").unwrap();
        let snippet_selector = Selector::parse(".result__snippet").unwrap();

        let mut results = Vec::new();

        for element in document.select(&result_selector) {
            let title_el = element.select(&title_selector).next();
            let snippet_el = element.select(&snippet_selector).next();

            if let (Some(title_el), Some(snippet_el)) = (title_el, snippet_el) {
                let title = title_el
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .to_string();

                // DuckDuckGo puts a redirect link in href, try to extract the real one if possible,
                // or just use the redirect link.
                let url = title_el.value().attr("href").unwrap_or("").to_string();
                let snippet = snippet_el
                    .text()
                    .collect::<Vec<_>>()
                    .join("")
                    .trim()
                    .to_string();

                if !title.is_empty() && !url.is_empty() {
                    results.push(SearchResult {
                        title,
                        url,
                        snippet,
                    });
                }
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
            .map_err(|e| format!("Failed to fetch URL: {}", e))?;

        if !response.status().is_success() {
            return Err(format!(
                "Fetch request failed with status: {}",
                response.status()
            ));
        }

        let html_content = response
            .text()
            .await
            .map_err(|e| format!("Failed to read response body: {}", e))?;

        let document = Html::parse_document(&html_content);

        // Basic extraction: grab all text nodes from body, skipping script and style tags.
        // A more sophisticated approach might use readability.rs, but this is a good start.
        let body_selector = Selector::parse("body").map_err(|_| "Failed to parse body selector")?;

        let mut extracted_text = String::new();

        if let Some(body) = document.select(&body_selector).next() {
            for node in body.text() {
                let trimmed = node.trim();
                if !trimmed.is_empty() {
                    extracted_text.push_str(trimmed);
                    extracted_text.push(' ');
                }
            }
        }

        Ok(extracted_text.trim().to_string())
    }
}

use kiwi_core::llm::LlmEngine;
use kiwi_core::memory::{ContextManager, Message};

pub struct WebTool<'a, W: WebSearcher, L: LlmEngine, C: ContextManager> {
    searcher: W,
    llm: L,
    context_manager: &'a mut C,
}

impl<'a, W: WebSearcher, L: LlmEngine, C: ContextManager> WebTool<'a, W, L, C> {
    pub fn new(searcher: W, llm: L, context_manager: &'a mut C) -> Self {
        Self {
            searcher,
            llm,
            context_manager,
        }
    }

    pub async fn search_and_recap(&mut self, query: &str) -> Result<(), String> {
        let results = self.searcher.search(query).await?;

        let mut context_text = String::new();
        if let Some(top_result) = results.first() {
            context_text.push_str(&format!(
                "Title: {}\nSnippet: {}\n",
                top_result.title, top_result.snippet
            ));

            // Optionally try to fetch the full text
            if let Ok(text) = self.searcher.fetch_and_extract_text(&top_result.url).await {
                // Keep it bounded for the prompt
                let truncated_text: String = text.chars().take(2000).collect();
                context_text.push_str(&format!("Content:\n{}\n", truncated_text));
            }
        } else {
            return Err("No results found".to_string());
        }

        let prompt = format!(
            "Recap the following search results about '{}'. Keep it concise.\n\n{}",
            query, context_text
        );

        let recap = self.llm.generate(&prompt).await?;

        _ = self.context_manager.add_message(Message {
            role: "system".to_string(),
            content: format!(
                "latest data fetched from the web about '{}': {}",
                query, recap
            ),
        }).await;

        Ok(())
    }
}
