use kiwi::memory::{ContextManager, MemoryBank, Message};

#[tokio::test]
async fn test_memory_bank_persistence() {
    let mut bank = MemoryBank::new(2048).await.unwrap();
    bank.clear().await.unwrap();

    bank.add_message(Message {
        role: "user".to_string(),
        content: "Hello!".to_string(),
        keywords: Some("greeting, hello".to_string()),
    })
    .await
    .unwrap();

    let prompt = bank.build_prompt(&["greeting".to_string()]);
    assert!(prompt.contains("Hello!"));
    assert!(prompt.contains("system")); // system prompt should be included
    assert!(prompt.contains("user"));
}
