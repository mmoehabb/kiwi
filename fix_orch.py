import re

with open("src/agents/orchestrator.rs", "r") as f:
    content = f.read()

# Add supervisor.store_context("user", text.to_string()) at the beginning of process_input
store_user = """    pub async fn process_input(&mut self, text: &str) -> (String, bool) {
        self.monitor.log("User Input", text);

        self.monitor.log("orchestrator to supervisor (context)", &format!("Store user context: {}", text));
        self.supervisor.store_context("user", text.to_string()).await;
"""
content = content.replace('    pub async fn process_input(&mut self, text: &str) -> (String, bool) {\n        self.monitor.log("User Input", text);', store_user)

# Add supervisor.store_context("assistant", final_response.clone()) at the end of process_input
store_assistant = """        self.monitor.log("orchestrator to speaker", &prompt);
        let final_response = self.speaker.generate_response(&prompt).await;
        self.monitor.log("Speaker Response", &final_response);

        self.monitor.log("orchestrator to supervisor (context)", &format!("Store assistant context: {}", final_response));
        self.supervisor.store_context("assistant", final_response.clone()).await;

        (final_response, exit_conversation)
    }"""
content = content.replace('        self.monitor.log("orchestrator to speaker", &prompt);\n        let final_response = self.speaker.generate_response(&prompt).await;\n        self.monitor.log("Speaker Response", &final_response);\n\n        (final_response, exit_conversation)\n    }', store_assistant)


with open("src/agents/orchestrator.rs", "w") as f:
    f.write(content)
