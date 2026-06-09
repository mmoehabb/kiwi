import sys

def modify():
    with open("src/config.rs", "r") as f:
        content = f.read()

    # Make the fields default on parse so `[permissions]` doesn't error out if fields are missing
    content = content.replace("    pub allowed_commands: Vec<String>,", "    #[serde(default)]\n    pub allowed_commands: Vec<String>,")
    content = content.replace("    pub allowed_read_paths: Vec<String>,", "    #[serde(default)]\n    pub allowed_read_paths: Vec<String>,")
    content = content.replace("    pub allowed_write_paths: Vec<String>,", "    #[serde(default)]\n    pub allowed_write_paths: Vec<String>,")

    with open("src/config.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
