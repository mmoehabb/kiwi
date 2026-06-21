use chrono::Local;
use std::collections::VecDeque;
use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::PathBuf;

#[derive(Clone)]
pub struct AgentsFlowMonitor {
    file_path: PathBuf,
}

impl AgentsFlowMonitor {
    pub fn new() -> Self {
        let mut path = dirs::data_local_dir().unwrap_or_else(|| {
            let mut home = dirs::home_dir().unwrap_or_else(|| PathBuf::from("."));
            home.push(".local");
            home.push("share");
            home
        });
        path.push("kiwi");
        fs::create_dir_all(&path).unwrap_or_default();
        path.push("agents_flow.txt");

        Self { file_path: path }
    }

    pub fn log(&self, component: &str, message: &str) {
        let now = Local::now();
        let timestamp = now.format("%Y-%m-%d %H:%M:%S").to_string();

        let single_line_message = message.trim().replace('\n', " ").replace('\r', "");
        let log_line = format!("[{}] {}: \"{}\"", timestamp, component, single_line_message);

        // Read existing lines
        let mut lines = VecDeque::new();
        if let Ok(file) = fs::File::open(&self.file_path) {
            let reader = BufReader::new(file);
            for line in reader.lines().flatten() {
                lines.push_back(line);
            }
        }

        // Add new line
        lines.push_back(log_line);

        // Keep only last 100 lines
        while lines.len() > 100 {
            lines.pop_front();
        }

        // Write back to file
        if let Ok(mut file) = OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .open(&self.file_path)
        {
            for line in lines {
                let _ = writeln!(file, "{}", line);
            }
        }
    }
}

impl Default for AgentsFlowMonitor {
    fn default() -> Self {
        Self::new()
    }
}
