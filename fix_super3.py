import re

with open("src/agents/supervisor.rs", "r") as f:
    content = f.read()

# Fix literal newline in string
replacement_eval = """            let prompt = format!(
                "Are the following two texts relevant to the same topic? \\
                Reply ONLY with 'Yes' or 'No'.\\n\\n\\
                Text 1: \\"{}\\"\\n\\
                Text 2: \\"{}\\"\\n\\n\\
                Output:",
                msg.content, text
            );"""

content = re.sub(r'            let prompt = format!\([\s\S]*?msg\.content, text\n            \);', replacement_eval, content)

with open("src/agents/supervisor.rs", "w") as f:
    f.write(content)
