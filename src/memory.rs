/// The Memory component handles conversation history, context windows, and intent routing.
/// It decides whether a prompt goes directly to the LLM or triggers tools like search/plugins.
use std::collections::VecDeque;

/// A single message in the conversation history.
#[derive(Debug, Clone)]
pub struct Message {
    pub role: String, // e.g., "user", "assistant", "system"
    pub content: String,
}

/// Trait defining the core logic for the context and memory system.
pub trait ContextManager {
    /// Appends a new message to the history.
    fn add_message(&mut self, message: Message);

    /// Formats the current history into a prompt string suitable for the specific LLM.
    /// TODO: Support different chat templates (e.g., Llama 2, ChatML).
    fn build_prompt(&self) -> String;

    /// Clears the memory context.
    fn clear(&mut self);
}

/// Trait defining how incoming user text is routed.
#[async_trait::async_trait]
pub trait IntentRouter {
    /// Determines the action required for the given input.
    /// TODO: Use a fast classifier or a strict LLM generation prompt to decide between
    /// direct chat, web search, or running a plugin/command.
    async fn route_intent(&self, transcribed_text: &str) -> Result<Intent, String>;
}

/// Enumerates the possible intents determined from user input.
#[derive(Debug, Clone)]
pub enum Intent {
    /// Simple conversational response.
    Chat,
    /// Requires up-to-date information.
    SearchRequired(String), // Contains search query
    /// Requires executing a plugin or command.
    ExecuteCommand(String), // Contains command identifier
}

/// The main struct managing short-term memory.
pub struct MemoryBank {
    pub history: VecDeque<Message>,
    pub max_tokens: usize,
    // TODO: Add tokenizer reference to accurately calculate prompt size.
}

impl MemoryBank {
    pub fn new(max_tokens: usize) -> Self {
        Self {
            history: VecDeque::new(),
            max_tokens,
        }
    }
}

impl ContextManager for MemoryBank {
    fn add_message(&mut self, message: Message) {
        self.history.push_back(message);
        // TODO: Implement sliding window logic to drop old messages if max_tokens is exceeded.
    }

    fn build_prompt(&self) -> String {
        // TODO: Concatenate messages using the correct chat template.
        let mut prompt = String::new();
        for msg in &self.history {
            prompt.push_str(&format!("{}: {}\n", msg.role, msg.content));
        }
        prompt
    }

    fn clear(&mut self) {
        self.history.clear();
    }
}
