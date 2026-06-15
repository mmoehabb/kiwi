use kiwi::intent::{Intent, IntentRouter, LlmIntentRouter};
use kiwi::llm::LlmEngine;

struct MockLlm;
#[async_trait::async_trait]
impl LlmEngine for MockLlm {
    async fn load_model(&mut self, _m: &str, _t: &str) -> Result<(), String> {
        Ok(())
    }
    async fn generate(&self, _p: &str) -> Result<String, String> {
        Ok("".to_string())
    }
    async fn generate_structured(&self, _p: &str) -> Result<String, String> {
        Ok(r#"{"type": "SearchRequired", "query": "test query"}"#.to_string())
    }
    async fn extract_keywords(&self, _t: &str) -> Result<Vec<String>, String> {
        Ok(vec![])
    }
}

#[tokio::test]
async fn test_intent_routing() {
    let llm = MockLlm;
    let router = LlmIntentRouter::new(&llm);

    let intent = router.route_intent("search something").await.unwrap();
    match intent {
        Intent::SearchRequired { query } => assert_eq!(query, "test query"),
        _ => panic!("Expected SearchRequired intent"),
    }
}

#[tokio::test]
async fn test_intent_routing_fallback() {
    struct MockLlmFallback;
    #[async_trait::async_trait]
    impl LlmEngine for MockLlmFallback {
        async fn load_model(&mut self, _m: &str, _t: &str) -> Result<(), String> {
            Ok(())
        }
        async fn generate(&self, _p: &str) -> Result<String, String> {
            Ok("".to_string())
        }
        async fn generate_structured(&self, _p: &str) -> Result<String, String> {
            Err("Failed to generate valid JSON".to_string())
        }
        async fn extract_keywords(&self, _t: &str) -> Result<Vec<String>, String> {
            Ok(vec![])
        }
    }

    let llm = MockLlmFallback;
    let router = LlmIntentRouter::new(&llm);

    let intent = router
        .route_intent("remember that I like apples")
        .await
        .unwrap_or(Intent::Chat);
    match intent {
        Intent::StoreMemory { content, keywords } => {
            assert_eq!(content, "remember that I like apples");
            assert_eq!(keywords, "remember, user");
        }
        _ => panic!("Expected StoreMemory intent"),
    }
}
