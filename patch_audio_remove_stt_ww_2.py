import sys

def modify():
    with open("src/audio.rs", "r") as f:
        lines = f.readlines()

    start_idx = -1
    end_idx = -1
    for i, line in enumerate(lines):
        if "        let wakeword_model_path" in line:
            start_idx = i
        if "        // 3. Initialize Piper TTS" in line:
            end_idx = i
            break

    if start_idx != -1 and end_idx != -1:
        del lines[start_idx:end_idx]

    with open("src/audio.rs", "w") as f:
        f.writelines(lines)

if __name__ == "__main__":
    modify()
