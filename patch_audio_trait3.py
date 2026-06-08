import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # Rename impl WakeWordEngine to impl WakeWordListener
    content = content.replace("impl WakeWordEngine for AudioManager {", "impl WakeWordListener for AudioManager {")

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
