use crate::llm::{LlmEngine, LocalLlm};
use crate::web::WebTool;
use std::sync::Arc;

pub struct Explorer {
    llm: Arc<LocalLlm>,
    web_tool: Arc<WebTool>,
}

impl Explorer {
    pub fn new(llm: Arc<LocalLlm>, web_tool: Arc<WebTool>) -> Self {
        Self { llm, web_tool }
    }

    pub async fn fetch_info(&self, query: &str) -> Result<String, String> {
        let results = self.web_tool.searcher.search(query).await?;

        if results.is_empty() {
            return Err(format!("No search results found for query: {}", query));
        }

        let first_result = &results[0];

        let mut url_to_fetch = first_result.url.clone();
        if url_to_fetch.starts_with("//") {
            url_to_fetch = format!("https:{}", url_to_fetch);
        }

        let extracted_text = match self
            .web_tool
            .searcher
            .fetch_and_extract_text(&url_to_fetch)
            .await
        {
            Ok(text) if !text.trim().is_empty() => text,
            _ => first_result.snippet.clone(),
        };

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
