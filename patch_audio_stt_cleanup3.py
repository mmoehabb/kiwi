import sys
import re

def modify():
    with open("src/audio.rs", "r") as f:
        content = f.read()

    # The regex didn't match the actual source code exactly for the wakeword_ctx init because it seems the code uses `WhisperContext::new_with_params`.
    # Let's find and remove it correctly.

    # We will just do:
    content = content.replace("        let tiny_model_path", "        /* let tiny_model_path")
    content = content.replace("            .map_err(|e| e.to_string())?;", "            .map_err(|e| e.to_string())?; */")

    # Let's just manually replace lines 85 to 97
    pass

if __name__ == "__main__":
    modify()
