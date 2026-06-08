import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # remove the extra Arc import
    content = content.replace("use std::sync::Arc;\nuse std::sync::Arc;", "use std::sync::Arc;")

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
