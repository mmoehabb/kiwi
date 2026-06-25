import re

with open("src/memory.rs", "r") as f:
    content = f.read()

# I will rewrite ContextManager and MemoryBank's build_prompt.
# I'll let ContextManager's `build_prompt` take an array of `Message`s and just format them.
# BUT wait, the orchestrator directly calls `build_prompt` on supervisor, which we haven't written yet.
# Let's write `MemoryBank::build_prompt` to accept `relevant_keywords` AND an optional boolean slice.
# Or I can just leave `build_prompt` as it is for memory.sqlite, and add a `build_context_prompt(&self, relevant_entries: &[bool])` for context.sqlite.
# Actually, the user asked to change the interface to:
# "Supervisor::build_prompt handles calling build_prompt on BOTH context_bank and memory_bank and combining them"

context_manager_replacement = """#[async_trait]
pub trait ContextManager {
    async fn add_message(&mut self, message: Message) -> Result<(), String>;
    fn build_prompt(&self, relevant_keywords: &[String], relevant_last_entries: &[bool]) -> String;
    fn build_prompt_from_bools(&self, is_relevant: &[bool]) -> String;
    async fn clear(&mut self) -> Result<(), String>;
}"""
content = re.sub(r"#\[async_trait\]\npub trait ContextManager \{.*?async fn clear\(&mut self\) -> Result<\(\), String>;\n\}", context_manager_replacement, content, flags=re.DOTALL)

build_prompt_from_bools = """    fn build_prompt_from_bools(&self, is_relevant: &[bool]) -> String {
        let mut prompt = String::new();
        for (i, msg) in self.history.iter().enumerate() {
            let mut relevant = false;
            if i < is_relevant.len() {
                relevant = is_relevant[i];
            }
            if relevant || msg.content == Self::SYSTEM_PROMPT {
                prompt.push_str(&format!(
                    "<|start_header_id|>{}<|end_header_id|>\\n\\n{}<|eot_id|>",
                    msg.role, msg.content
                ));
            }
        }
        prompt
    }"""
content = content.replace("    async fn clear(&mut self) -> Result<(), String> {", build_prompt_from_bools + "\n\n    async fn clear(&mut self) -> Result<(), String> {")

with open("src/memory.rs", "w") as f:
    f.write(content)
