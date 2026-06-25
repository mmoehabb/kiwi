import re

with open("src/agents/orchestrator.rs", "r") as f:
    content = f.read()

# Fix Orchestrator's calls to Supervisor
content = content.replace("self.supervisor.memory_bank.build_prompt", "self.supervisor.build_prompt")

with open("src/agents/orchestrator.rs", "w") as f:
    f.write(content)
