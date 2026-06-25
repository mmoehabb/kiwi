import re

with open("src/main.rs", "r") as f:
    content = f.read()

# I previously changed main.rs to:
# let memory_bank = MemoryBank::new(2048, "memory.sqlite", 50).await.expect("Failed to initialize memory bank");

main_replace = """    let memory_bank = MemoryBank::new(2048, "memory.sqlite", 50)
        .await
        .expect("Failed to initialize memory bank");

    let context_bank = MemoryBank::new(2048, "context.sqlite", config.app.context_max_rows)
        .await
        .expect("Failed to initialize context bank");"""

content = re.sub(r"    let memory_bank = MemoryBank::new.*?expect\(\"Failed to initialize memory bank\"\);", main_replace, content, flags=re.DOTALL)

content = content.replace("Supervisor::new(supervisor_llm, memory_bank);", "Supervisor::new(supervisor_llm, memory_bank, context_bank);")

with open("src/main.rs", "w") as f:
    f.write(content)
