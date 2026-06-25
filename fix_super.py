import re

with open("src/agents/supervisor.rs", "r") as f:
    content = f.read()

# Fix the extra brace at line 15
content = content.replace("    pub fn new(llm: Arc<LocalLlm>, memory_bank: MemoryBank, context_bank: MemoryBank) -> Self {\n        Self { llm, memory_bank, context_bank }\n    }\n    }", "    pub fn new(llm: Arc<LocalLlm>, memory_bank: MemoryBank, context_bank: MemoryBank) -> Self {\n        Self { llm, memory_bank, context_bank }\n    }")

with open("src/agents/supervisor.rs", "w") as f:
    f.write(content)
