import re

with open("src/main.rs", "r") as f:
    content = f.read()

content = content.replace("MemoryBank::new(2048)", 'MemoryBank::new(2048, "memory.sqlite", 50)')

with open("src/main.rs", "w") as f:
    f.write(content)
