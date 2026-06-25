import re

with open("tests/memory_tests.rs", "r") as f:
    content = f.read()

content = content.replace("MemoryBank::new(2048)", 'MemoryBank::new(2048, "test_memory.sqlite", 50)')

with open("tests/memory_tests.rs", "w") as f:
    f.write(content)
