use kiwi::web::{WebClient, WebTool};
use kiwi_core::llm::LlmEngine;
use kiwi_core::web::WebSearcher;
use kiwi_core::memory::ContextManager;
use kiwi::memory::MemoryBank;
use mockito::Server;

// Mock LlmEngine
struct MockLlm;
#[async_trait::async_trait]
impl LlmEngine for MockLlm {
    async fn load_model(&mut self, _model_path: &str, _tokenizer_path: &str) -> Result<(), String> {
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
    let mut memory = MemoryBank::new(1000).await.unwrap();

    // Clear history to start with 1 element (the system prompt added by `new()`)
    memory.clear().await.unwrap();
    let initial_len = memory.history.len(); // normally 1 for system prompt

    let mut tool = WebTool::new(client, llm, &mut memory);

    tool.search_and_recap("recap test").await.unwrap();

    search_mock.assert_async().await;
    content_mock.assert_async().await;

    // Assert memory was updated correctly

    assert_eq!(memory.history.len(), initial_len + 1);

    let msg = &memory.history[memory.history.len() - 1];

    assert_eq!(msg.role, "system");
    assert!(msg.content.starts_with("latest data fetched from the web about 'recap test': Mock recap for prompt: Recap the following"));
}
