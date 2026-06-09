import sys

def modify():
    with open("src/wakeword.rs", "r") as f:
        content = f.read()

    # Add `remove_template` to WakewordEngine
    old = """    pub fn clear_templates(&mut self) {
        self.templates.clear();
    }"""

    new = """    pub fn clear_templates(&mut self) {
        self.templates.clear();
    }

    pub fn remove_template(&mut self, index: usize) {
        if index < self.templates.len() {
            self.templates.remove(index);
        }
    }"""
    content = content.replace(old, new)

    with open("src/wakeword.rs", "w") as f:
        f.write(content)

if __name__ == "__main__":
    modify()
