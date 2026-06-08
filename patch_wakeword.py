import sys

def modify():
    with open("src/wakeword.rs", "r") as f:
        content = f.read()

    # The type inference issue
    content = content.replace("let mag = c.norm();", "let mag: f32 = c.norm();")

    with open("src/wakeword.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
