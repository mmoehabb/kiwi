import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    content = content.replace("            wakeword_ctx: Arc::new(wakeword_ctx),", "            _wakeword_ctx: Arc::new(wakeword_ctx),")
    content = content.replace("            config,\n        }", "            _config: config,\n        }")

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
