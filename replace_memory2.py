import re

with open("src/memory.rs", "r") as f:
    content = f.read()

# I am changing `build_prompt` to just format the given list of messages in a string.
# Since the supervisor now has to manage evaluating both context and memory and merging them,
# it's best if we let MemoryBank have a method to format specific entries.
# Wait, actually, the user said:
# "You should change the interface so that Supervisor::build_prompt handles calling build_prompt on BOTH context_bank and memory_bank and combining them."

# To make this easy, I'll update `MemoryBank::build_prompt` to take a list of booleans `is_relevant: &[bool]` representing exactly the elements in `self.history`.
# If `is_relevant.len() != self.history.len()`, we default to treating it as false.
# Oh, wait. `build_prompt` on memory bank used `relevant_keywords` for long-term memory. Let's just create a `build_prompt` for memory and one for context?
# Since `MemoryBank` is the same struct for both, I will change `build_prompt` in `MemoryBank` to:
# fn build_prompt_with_indices(&self, relevant_indices: &[usize]) -> String;
# Or even better, `ContextManager` interface.
# Let's just redefine `build_prompt` on `MemoryBank` to format a vector of references to `Message`.
