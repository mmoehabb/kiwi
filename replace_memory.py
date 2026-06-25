import re

with open("src/memory.rs", "r") as f:
    content = f.read()

# Update MemoryBank struct
struct_replacement = """pub struct MemoryBank {
    pub history: VecDeque<Message>,
    pub max_tokens: usize,
    pub max_rows: usize,
    pub db_pool: SqlitePool,
}"""
content = re.sub(r"pub struct MemoryBank \{.*?\bdb_pool: SqlitePool,\n\}", struct_replacement, content, flags=re.DOTALL)

# Update MemoryBank::new
new_fn_replacement = """    pub async fn new(max_tokens: usize, db_name: &str, max_rows: usize) -> Result<Self, String> {
        // Ensure config directory exists
        let config_dir = dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("kiwi");
        if !config_dir.exists() {
            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;
        }

        let db_path = config_dir.join(db_name);"""
content = content.replace("    pub async fn new(max_tokens: usize) -> Result<Self, String> {\n        // Ensure config directory exists\n        let config_dir = dirs::config_dir()\n            .unwrap_or_else(|| PathBuf::from(\".\"))\n            .join(\"kiwi\");\n        if !config_dir.exists() {\n            std::fs::create_dir_all(&config_dir).map_err(|e| e.to_string())?;\n        }\n\n        let db_path = config_dir.join(\"memory.sqlite\");", new_fn_replacement)

# Update return OK
ok_replacement = """        Ok(Self {
            history,
            max_tokens,
            max_rows,
            db_pool: pool,
        })"""
content = content.replace("        Ok(Self {\n            history,\n            max_tokens,\n            db_pool: pool,\n        })", ok_replacement)

# Update add_message while loop
add_msg_replacement = "        while self.history.len() > self.max_rows {"
content = content.replace("        while self.history.len() > 50 {", add_msg_replacement)

with open("src/memory.rs", "w") as f:
    f.write(content)
