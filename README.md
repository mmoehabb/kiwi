# Kiwi 🦜

Kiwi is a lightweight, fully local AI assistant designed for your desktop. Powered by small Large Language Models (1B - 9B parameters), Kiwi ensures your data stays private while offering a responsive, intelligent, and delightfully playful companion.

Just say **"Hey Kiwi"** to wake up your assistant. Kiwi will listen to your voice, process your requests, and reply with its signature friendly parrot voice. Beyond just answering questions, Kiwi is equipped to search the web, stay up-to-date on current topics, and, via a flexible permission system, interact with your computer.

## Features

- **100% Local Execution**: Runs entirely on your hardware using small, efficient LLMs.
- **Voice Activated**: Always listening for the "Hey Kiwi" wake word.
- **Speech-to-Text & Text-to-Speech**: Seamlessly understands your voice and responds in a unique parrot persona.
- **Internet Search**: Automatically uses Google Search and web scraping to find up-to-date information on the web.
- **Extensible via Plugins**: Write powerful plugins using the lightweight [Rhai](https://rhai.rs/) scripting language to teach Kiwi new tricks.
- **Graphical Mascot**: A playful parrot pops up on your screen when Kiwi is actively listening or speaking.

### Upcoming Features
- **App & Command Execution**: Allow Kiwi to run specific terminal commands or open applications based on a granular configuration file.
- **File System Access**: Grant Kiwi the ability to read and write text files.

---

## Getting Started

### Prerequisites

To build and run Kiwi, you will need the following installed on your system:

- **Rust** (1.70 or newer)
- **Cargo** (comes with Rust)
- Audio drivers (e.g., ALSA/PulseAudio for Linux, CoreAudio for macOS, WASAPI for Windows)
- SSL/TLS libraries (for internet search and web scraping)

**System Dependencies (Linux/Ubuntu):**
To compile the audio and AI dependencies, you will need a few system libraries installed:
```bash
sudo apt-get update && sudo apt-get install -y libasound2-dev pkg-config cmake clang curl build-essential wget
```

### Installation & Build

1. Clone the repository:
   ```bash
   git clone https://github.com/your-org/kiwi.git
   cd kiwi
   ```

2. Build the project using Cargo:
   ```bash
   cargo build --release
   ```

3. Download the necessary LLM model weights (e.g., GGUF files) and audio models into the `models/` directory. *(Note: Specific model download links will be provided in future releases).*

### Running Kiwi

To start Kiwi in background daemon mode:

```bash
cargo run --release
```

When Kiwi starts, it runs silently in the background, continuously listening for the wake word.

---

## How to Use Kiwi

1. **Activate**: Say loudly and clearly, **"Hey Kiwi"**.
2. **Interact**: A playful graphical parrot will pop up on your screen. Speak your question or command.
3. **Response**: Kiwi will process your prompt through the local LLM (and search the web if necessary) and respond back using its text-to-speech parrot voice.

### Plugins

Kiwi is designed to be highly extensible. Plugins are written in **Rhai**, a safe and fast scripting language for Rust.

To add a plugin:
1. Write your `.rhai` script.
2. Place it in the `~/.config/kiwi/plugins/` directory.
3. Restart Kiwi or ask Kiwi to "reload plugins".

### Permissions

Security is a priority. For commands and file operations, Kiwi relies on a strict configuration file located at `~/.config/kiwi/permissions.toml`. By default, Kiwi has no permission to execute shell commands or modify your file system until you explicitly allow it (e.g., allowing `git *` or specific file paths).

## License

This project is licensed under the MIT License. See the [LICENSE](LICENSE) file for details.
