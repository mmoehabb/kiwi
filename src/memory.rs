use kiwi_core::memory::{ContextManager, Message};
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::path::PathBuf;

/// The main struct managing memory with SQLite persistence.
pub struct MemoryBank {
    pub history: VecDeque<Message>,
    pub max_tokens: usize,
    pub db_pool: SqlitePool,
}

impl MemoryBank {
    pub const SYSTEM_PROMPT: &'static str = "You are Kiwi, a friendly, playful, and helpful local AI assistant. You take the persona of a stylized, modern parrot. Your voice is conversational, warm, and inviting. You embrace your parrot persona playfully without being annoying. Be concise and brief in your responses. Say 'Squawk!' occasionally or when an error occurs. Protect user privacy and help them effectively.";

    pub async fn new(max_tokens: usize) -> Result<Self, String> {
        // Ensure config directory exists
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kiwi");
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        }

        let db_path = config_dir.join("memory.sqlite");
        use sqlx::sqlite::SqliteConnectOptions;
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| e.to_string())?;

        // Create table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        // Load existing history
        let rows = sqlx::query("SELECT role, content FROM messages ORDER BY id ASC")
            .fetch_all(&pool)
            .await
            .map_err(|e| e.to_string())?;

        let mut history = VecDeque::new();
        // Always start with system prompt if no system prompt in history
        let mut has_system = false;

        for row in rows {
            let role: String = row.get("role");
            if role == "system" {
                has_system = true;
            }
            let content: String = row.get("content");
            history.push_back(Message { role, content });
        }

        if !has_system {
            let sys_msg = Message {
                role: "system".to_string(),
                content: Self::SYSTEM_PROMPT.to_string(),
            };
            history.push_back(sys_msg.clone());

            sqlx::query("INSERT INTO messages (role, content) VALUES (?, ?)")
                .bind(&sys_msg.role)
                .bind(&sys_msg.content)
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(Self {
            history,
            max_tokens,
            db_pool: pool,
        })
    }
}

#[async_trait::async_trait]
impl ContextManager for MemoryBank {
    async fn add_message(&mut self, message: Message) -> Result<(), String> {
        self.history.push_back(message.clone());

        sqlx::query("INSERT INTO messages (role, content) VALUES (?, ?)")
            .bind(&message.role)
            .bind(&message.content)
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        // Basic sliding window based on message count for now
        // A more advanced version would use actual tokenization
        while self.history.len() > 50 {
            // keep the system prompt
            if let Some(_msg) = self.history.get(1) {
                // Delete the oldest non-system message
                let _ = sqlx::query("DELETE FROM messages WHERE id IN (SELECT id FROM messages WHERE role != 'system' ORDER BY id ASC LIMIT 1)")
                    .execute(&self.db_pool)
                    .await;
                self.history.remove(1);
            }
        }

        Ok(())
    }

    fn build_prompt(&self) -> String {
        // Llama 3 prompt format:
        // <|begin_of_text|><|start_header_id|>system<|end_header_id|>
        // {{ system_prompt }}<|eot_id|><|start_header_id|>user<|end_header_id|>
        // {{ user_message }}<|eot_id|><|start_header_id|>assistant<|end_header_id|>

        let mut prompt = String::from("<|begin_of_text|>");
        for msg in &self.history {
            prompt.push_str(&format!(
                "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
                msg.role, msg.content
            ));
        }
        prompt.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
        prompt
    }

    async fn clear(&mut self) -> Result<(), String> {
        self.history.clear();
        sqlx::query("DELETE FROM messages")
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        // Re-add system prompt
        let sys_msg = Message {
            role: "system".to_string(),
            content: Self::SYSTEM_PROMPT.to_string(),
        };
        self.history.push_back(sys_msg.clone());

        sqlx::query("INSERT INTO messages (role, content) VALUES (?, ?)")
            .bind(&sys_msg.role)
            .bind(&sys_msg.content)
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
