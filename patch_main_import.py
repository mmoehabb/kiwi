import sys

def modify():
    with open("src/main.rs", "r") as f:
        content = f.read()

    # The trait name changed from WakeWordEngine to WakeWordListener in the top-level import
    content = content.replace("use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordEngine};", "use kiwi::audio::{AudioManager, SpeechToText, TextToSpeech, WakeWordListener};")

    with open("src/main.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
