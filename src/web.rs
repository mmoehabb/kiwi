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

use reqwest::Client;
use scraper::{Html, Selector};

/// The main struct handling outgoing web requests.
pub struct WebClient {
    client: Client,
    search_base_url: String,
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
    #[cfg(test)]
    pub(crate) fn with_base_url(base_url: &str) -> Self {
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
use crate::llm::LlmEngine;
use crate::memory::{ContextManager, Message};

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

        self.context_manager.add_message(Message {
            role: "system".to_string(),
            content: format!(
                "latest data fetched from the web about '{}': {}",
                query, recap
            ),
        });

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::memory::MemoryBank;
    use mockito::Server;

    // Mock LlmEngine
    struct MockLlm;
    #[async_trait::async_trait]
    impl LlmEngine for MockLlm {
        async fn load_model(&mut self, _model_path: &str) -> Result<(), String> {
            Ok(())
        }

        async fn generate(&self, prompt: &str) -> Result<String, String> {
            Ok(format!(
                "Mock recap for prompt: {}",
                prompt.chars().take(20).collect::<String>()
            ))
        }

        async fn generate_structured(&self, _prompt: &str) -> Result<String, String> {
            Ok("".to_string())
        }
    }

    #[tokio::test]
    async fn test_web_client_search() {
        let mut server = Server::new_async().await;

        let mock_html = r#"
        <html>
            <body>
                <div class="result">
                    <h2 class="result__title">
                        <a class="result__a" href="https://example.com/mock-result">Mock Result Title</a>
                    </h2>
                    <a class="result__snippet">This is a mock snippet for testing.</a>
                </div>
            </body>
        </html>
        "#;

        let mock = server
            .mock("GET", "/html/?q=test%20query")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(mock_html)
            .create_async()
            .await;

        let client = WebClient::with_base_url(&format!("{}/html/", server.url()));

        let results = client.search("test query").await.unwrap();

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].title, "Mock Result Title");
        assert_eq!(results[0].url, "https://example.com/mock-result");
        assert_eq!(results[0].snippet, "This is a mock snippet for testing.");

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_web_client_fetch_text() {
        let mut server = Server::new_async().await;

        let mock_html = r#"
        <html>
            <body>
                <h1>Main Heading</h1>
                <p>This is the content of the paragraph.</p>
                <script>console.log('Ignore me');</script>
            </body>
        </html>
        "#;

        let mock = server
            .mock("GET", "/test-page")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(mock_html)
            .create_async()
            .await;

        let client = WebClient::new(); // Base url doesn't matter for fetch_and_extract_text

        let text = client
            .fetch_and_extract_text(&format!("{}/test-page", server.url()))
            .await
            .unwrap();

        assert!(text.contains("Main Heading"));
        assert!(text.contains("This is the content of the paragraph."));
        // Depending on scraper text extraction it might include script contents or whitespace

        mock.assert_async().await;
    }

    #[tokio::test]
    async fn test_web_tool_search_and_recap() {
        let mut server = Server::new_async().await;
        let server_url = server.url();

        let mock_search_html = format!(
            r#"
        <html>
            <body>
                <div class="result">
                    <h2 class="result__title">
                        <a class="result__a" href="{}/test-content">Recap Test Title</a>
                    </h2>
                    <a class="result__snippet">Recap snippet.</a>
                </div>
            </body>
        </html>
        "#,
            server_url
        );

        let search_mock = server
            .mock("GET", "/html/?q=recap%20test")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body(mock_search_html)
            .create_async()
            .await;

        let content_mock = server
            .mock("GET", "/test-content")
            .with_status(200)
            .with_header("content-type", "text/html")
            .with_body("<html><body><p>Detailed fetched content</p></body></html>")
            .create_async()
            .await;

        let client = WebClient::with_base_url(&format!("{}/html/", server_url));
        let llm = MockLlm;
        let mut memory = MemoryBank::new(1000);

        let mut tool = WebTool::new(client, llm, &mut memory);

        tool.search_and_recap("recap test").await.unwrap();

        search_mock.assert_async().await;
        content_mock.assert_async().await;

        // Assert memory was updated correctly
        assert_eq!(memory.history.len(), 1);
        let msg = &memory.history[0];
        assert_eq!(msg.role, "system");
        assert!(msg.content.starts_with("latest data fetched from the web about 'recap test': Mock recap for prompt: Recap the following"));
    }
}
