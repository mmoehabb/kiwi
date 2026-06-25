import re

with open("src/config.rs", "r") as f:
    content = f.read()

# Add default function for context_max_rows
if "fn default_context_max_rows" not in content:
    default_fn = """fn default_num_ctx() -> u32 {
    32768
}

fn default_context_max_rows() -> usize {
    15
}"""
    content = content.replace("fn default_num_ctx() -> u32 {\n    32768\n}", default_fn)

# Add field to AppConfig
if "pub context_max_rows: usize" not in content:
    field = """    #[serde(default = "default_num_ctx")]
    pub num_ctx: u32,

    #[serde(default = "default_context_max_rows")]
    pub context_max_rows: usize,"""
    content = content.replace('    #[serde(default = "default_num_ctx")]\n    pub num_ctx: u32,', field)

# Add to Default implementation
if "context_max_rows: default_context_max_rows()" not in content:
    default_impl = """            num_ctx: default_num_ctx(),
            context_max_rows: default_context_max_rows(),"""
    content = content.replace("            num_ctx: default_num_ctx(),", default_impl)

with open("src/config.rs", "w") as f:
    f.write(content)
