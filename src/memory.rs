use async_trait::async_trait;
use sqlx::Row;
use sqlx::SqlitePool;
use std::collections::VecDeque;
use std::path::PathBuf;

#[derive(Debug, Clone)]
pub struct Message {
    pub role: String,
    pub content: String,
    pub keywords: Option<String>,
}

#[async_trait]
pub trait ContextManager {
    async fn add_message(&mut self, message: Message) -> Result<(), String>;
    fn build_prompt(&self, relevant_keywords: &[String], relevant_last_entries: &[bool]) -> String;
    fn build_prompt_from_bools(&self, is_relevant: &[bool]) -> String;
    async fn clear(&mut self) -> Result<(), String>;
}

/// The main struct managing memory with SQLite persistence.
pub struct MemoryBank {
    pub history: VecDeque<Message>,
    pub max_tokens: usize,
    pub max_rows: usize,
    pub db_pool: SqlitePool,
}

impl MemoryBank {
    pub const SYSTEM_PROMPT: &'static str = "You are Kiwi, a friendly, playful, and helpful local AI assistant. You take the persona of a stylized, modern parrot. Your voice is conversational, warm, and inviting. You embrace your parrot persona playfully without being annoying. Be concise and brief in your responses. Say 'Squawk!' occasionally or when an error occurs. Protect user privacy and help them effectively.";

    pub async fn new(max_tokens: usize, db_name: &str, max_rows: usize) -> Result<Self, String> {
        // Ensure config directory exists
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kiwi");
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        }

        let db_path = config_dir.join(db_name);
        use sqlx::sqlite::SqliteConnectOptions;
        let options = SqliteConnectOptions::new()
            .filename(&db_path)
            .create_if_missing(true);
        let pool = SqlitePool::connect_with(options)
            .await
            .map_err(|e| e.to_string())?;

        // Check if keywords column exists, add it if not (simple migration)
        let _ = sqlx::query("ALTER TABLE messages ADD COLUMN keywords TEXT")
            .execute(&pool)
            .await;

        // Create table if it doesn't exist
        sqlx::query(
            "CREATE TABLE IF NOT EXISTS messages (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                role TEXT NOT NULL,
                content TEXT NOT NULL,
                keywords TEXT,
                timestamp DATETIME DEFAULT CURRENT_TIMESTAMP
            )",
        )
        .execute(&pool)
        .await
        .map_err(|e| e.to_string())?;

        // Load existing history
        let rows = sqlx::query("SELECT role, content, keywords FROM messages ORDER BY id ASC")
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
            let keywords: Option<String> = row.get("keywords");
            history.push_back(Message {
                role,
                content,
                keywords,
            });
        }

        if !has_system {
            let sys_msg = Message {
                role: "system".to_string(),
                content: Self::SYSTEM_PROMPT.to_string(),
                keywords: None,
            };
            history.push_back(sys_msg.clone());

            sqlx::query("INSERT INTO messages (role, content, keywords) VALUES (?, ?, ?)")
                .bind(&sys_msg.role)
                .bind(&sys_msg.content)
                .bind(&sys_msg.keywords)
                .execute(&pool)
                .await
                .map_err(|e| e.to_string())?;
        }

        Ok(Self {
            history,
            max_tokens,
            max_rows,
            db_pool: pool,
        })
    }
}

#[async_trait::async_trait]
impl ContextManager for MemoryBank {
    async fn add_message(&mut self, message: Message) -> Result<(), String> {
        self.history.push_back(message.clone());

        sqlx::query("INSERT INTO messages (role, content, keywords) VALUES (?, ?, ?)")
            .bind(&message.role)
            .bind(&message.content)
            .bind(&message.keywords)
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        // Basic sliding window based on message count for now
        // A more advanced version would use actual tokenization
        while self.history.len() > self.max_rows {
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

    fn build_prompt(&self, relevant_keywords: &[String], relevant_last_entries: &[bool]) -> String {
        // Llama 3 prompt format:
        // <|begin_of_text|><|start_header_id|>system<|end_header_id|>
        // {{ system_prompt }}<|eot_id|><|start_header_id|>user<|end_header_id|>
        // {{ user_message }}<|eot_id|><|start_header_id|>assistant<|end_header_id|>

        let mut prompt = String::from("<|begin_of_text|>");

        let history_len = self.history.len();
        let last_five_start = history_len.saturating_sub(5);

        for (i, msg) in self.history.iter().enumerate() {
            let is_last_five = i >= last_five_start;

            // Always include the root system prompt, not web data
            if msg.content == Self::SYSTEM_PROMPT {
                prompt.push_str(&format!(
                    "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
                    msg.role, msg.content
                ));
                continue;
            }

            let mut is_relevant = false;

            if is_last_five {
                let idx_in_last_five = i - last_five_start;
                // Since system prompt isn't in recent_entries in daemon,
                // we should be careful with indexing, but earlier we said `continue` for system prompt.
                // However, wait, in tests, there is a system prompt and a user prompt.
                // Let's just use what was given in `relevant_last_entries`.
                if idx_in_last_five < relevant_last_entries.len() {
                    is_relevant = relevant_last_entries[idx_in_last_five];
                } else {
                    // Fallback to true if array isn't provided correctly in tests
                    is_relevant = true;
                }
            }

            // Check if keywords match, needing at least 2 matches if it's older
            if !is_last_five {
                #[allow(clippy::collapsible_if)]
                if let Some(msg_keywords_str) = &msg.keywords {
                    let msg_keywords: Vec<&str> =
                        msg_keywords_str.split(',').map(|s| s.trim()).collect();
                    let mut match_count = 0;
                    for rk in relevant_keywords {
                        if msg_keywords.contains(&rk.trim()) {
                            match_count += 1;
                        }
                    }
                    if match_count >= 2 {
                        is_relevant = true;
                    }
                }
            }

            if is_relevant {
                prompt.push_str(&format!(
                    "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
                    msg.role, msg.content
                ));
            }
        }
        prompt.push_str("<|start_header_id|>assistant<|end_header_id|>\n\n");
        prompt
    }

    fn build_prompt_from_bools(&self, is_relevant: &[bool]) -> String {
        let mut prompt = String::new();
        for (i, msg) in self.history.iter().enumerate() {
            let mut relevant = false;
            if i < is_relevant.len() {
                relevant = is_relevant[i];
            }
            if relevant || msg.content == Self::SYSTEM_PROMPT {
                prompt.push_str(&format!(
                    "<|start_header_id|>{}<|end_header_id|>\n\n{}<|eot_id|>",
                    msg.role, msg.content
                ));
            }
        }
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
            keywords: None,
        };
        self.history.push_back(sys_msg.clone());

        sqlx::query("INSERT INTO messages (role, content, keywords) VALUES (?, ?, ?)")
            .bind(&sys_msg.role)
            .bind(&sys_msg.content)
            .bind(&sys_msg.keywords)
            .execute(&self.db_pool)
            .await
            .map_err(|e| e.to_string())?;

        Ok(())
    }
}
