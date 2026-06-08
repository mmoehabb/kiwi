import sys

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # To avoid dead code warning, we can prepend `_` to wakeword_ctx and config in AudioManager struct definition and where it is instantiated.
    content = content.replace("    wakeword_ctx: Arc<WhisperContext>,", "    _wakeword_ctx: Arc<WhisperContext>,")
    content = content.replace("    config: Arc<Configuration>,", "    _config: Arc<Configuration>,")
    content = content.replace("            wakeword_ctx: whisper_ctx.clone(),", "            _wakeword_ctx: whisper_ctx.clone(),")
    content = content.replace("            config: config.clone(),", "            _config: config.clone(),")

    with open("src/audio.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
