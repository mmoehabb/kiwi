use kiwi::intent::LlmIntentRouter;
use kiwi_core::intent::Intent;
use kiwi_core::intent::IntentRouter;

struct MockLlm;
#[async_trait::async_trait]
impl kiwi_core::llm::LlmEngine for MockLlm {
    async fn load_model(&mut self, _m: &str, _t: &str) -> Result<(), String> {
        Ok(())
    }
    async fn generate(&self, _p: &str) -> Result<String, String> {
        Ok("".to_string())
    }
    async fn generate_structured(&self, _p: &str) -> Result<String, String> {
        Ok(r#"{"type": "SearchRequired", "query": "test query"}"#.to_string())
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
