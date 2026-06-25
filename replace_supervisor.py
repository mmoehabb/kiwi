import re

with open("src/agents/supervisor.rs", "r") as f:
    content = f.read()

# Modify Supervisor struct
supervisor_struct = """pub struct Supervisor {
    llm: Arc<LocalLlm>,
    pub memory_bank: MemoryBank,
    pub context_bank: MemoryBank,
}"""
content = re.sub(r"pub struct Supervisor \{.*?pub memory_bank: MemoryBank,\n\}", supervisor_struct, content, flags=re.DOTALL)

# Modify Supervisor::new
new_fn = """    pub fn new(llm: Arc<LocalLlm>, memory_bank: MemoryBank, context_bank: MemoryBank) -> Self {
        Self { llm, memory_bank, context_bank }
    }"""
content = re.sub(r"    pub fn new\(.*?\) -> Self \{.*?\}", new_fn, content, flags=re.DOTALL)

# Add store_context
store_context = """    pub async fn store_context(&mut self, role: &str, content: String) {
        let _ = self
            .context_bank
            .add_message(Message {
                role: role.to_string(),
                content,
                keywords: None,
            })
            .await;
    }"""

# Add build_prompt
build_prompt = """    pub fn build_prompt(&self, current_keywords: &[String], context_relevance: &[bool]) -> String {
        // We will build the context prompt using boolean values for recent entries.
        let mut context_prompt = self.context_bank.build_prompt_from_bools(context_relevance);

        // Remove trailing assistant header if present, to merge memory prompt properly.
        if context_prompt.ends_with("<|start_header_id|>assistant<|end_header_id|>\\n\\n") {
            context_prompt.truncate(context_prompt.len() - "<|start_header_id|>assistant<|end_header_id|>\\n\\n".len());
        }

        // Use memory bank purely for keyword matches, we pass an empty slice for relevant_last_entries
        // to bypass the last_five logic for memory_bank, since we don't need it.
        let mut memory_prompt = self.memory_bank.build_prompt(current_keywords, &[]);

        // Strip the beginning of the text to append to context
        if memory_prompt.starts_with("<|begin_of_text|>") {
            memory_prompt = memory_prompt["<|begin_of_text|>".len()..].to_string();
        }

        format!("{}{}", context_prompt, memory_prompt)
    }"""

# Update evaluate_relevance to use context_bank, and evaluate ALL entries.
eval_rel = """    pub async fn evaluate_relevance(&self, text: &str) -> Vec<bool> {
        let mut results = Vec::new();

        for msg in self.context_bank.history.iter() {
            if msg.role == "system" {
                results.push(false);
                continue;
            }

            let prompt = format!(
                "Are the following two texts relevant to the same topic? \\
                Reply ONLY with 'Yes' or 'No'.\\n\\n\\
                Text 1: \\"{}\\"\\n\\
                Text 2: \\"{}\\"\\n\\n\\
                Output:",
                msg.content, text
            );

            let is_relevant = self
                .llm
                .generate(&prompt)
                .await
                .unwrap_or_default()
                .trim()
                .to_lowercase()
                .contains("yes");

            results.push(is_relevant);
        }
        results
    }"""

content = re.sub(r"    pub async fn evaluate_relevance.*?\n    }", eval_rel, content, flags=re.DOTALL)
content = content.replace("pub async fn store_memory", store_context + "\n\n    pub async fn store_memory")
content = content.replace("    pub async fn extract_keywords", build_prompt + "\n\n    pub async fn extract_keywords")

with open("src/agents/supervisor.rs", "w") as f:
    f.write(content)
