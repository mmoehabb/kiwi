use crate::llm::LocalLlm;
use crate::web::WebTool;
use std::sync::Arc;

pub struct Explorer {
    _llm: Arc<LocalLlm>,
    web_tool: Arc<WebTool>,
}

impl Explorer {
    pub fn new(llm: Arc<LocalLlm>, web_tool: Arc<WebTool>) -> Self {
        Self {
            _llm: llm,
            web_tool,
        }
    }

    pub async fn fetch_info(&self, query: &str) -> Result<String, String> {
        self.web_tool.search_and_recap(query).await
    }
}
