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

    // There are 2 messages in memory: system and the user message we added.
    // The history length is 2. last_five_start is 0.
    // The user message is at index 1. So it corresponds to idx_in_last_five = 1.
    // Therefore, we pass `&[true, true]` so index 1 gets evaluated as `true`.
    let prompt = bank.build_prompt(&["greeting".to_string()], &[true, true]);
    assert!(prompt.contains("Hello!"));
    assert!(prompt.contains("system")); // system prompt should be included
    assert!(prompt.contains("user"));
}
