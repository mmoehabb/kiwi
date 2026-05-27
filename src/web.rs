use kiwi_core::web::{SearchResult, WebSearcher};

/// The main struct handling outgoing web requests.
pub struct WebClient {
    // TODO: Add HTTP client instance here.
}

impl WebClient {
    pub fn new() -> Self {
        Self {}
    }
}

#[async_trait::async_trait]
impl WebSearcher for WebClient {
    async fn search(&self, _query: &str) -> Result<Vec<SearchResult>, String> {
        // TODO: Construct search URL, perform GET request, parse results.
        Ok(vec![])
    }

    async fn fetch_and_extract_text(&self, _url: &str) -> Result<String, String> {
        // TODO: Perform GET request, extract text nodes, strip HTML tags.
        Ok("Extracted web text goes here.".to_string())
    }
}
