//! The Config component manages `config.toml`, `permissions.toml` and general application settings.
//! It enforces the strict, whitelist-based security model.

use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Represents the permissions granted to Kiwi by the user.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct PermissionsConfig {
    /// List of allowed shell commands (can include wildcards like `git *`).
    #[serde(default)]
    pub allowed_commands: Vec<String>,
    /// List of directories Kiwi is allowed to read from.
    #[serde(default)]
    pub allowed_read_paths: Vec<String>,
    /// List of directories Kiwi is allowed to write to.
    #[serde(default)]
    pub allowed_write_paths: Vec<String>,
}

fn default_model_name() -> String {
    "qwen2.5-1.5b-instruct".to_string()
}

fn default_wake_word() -> String {
    "hey kiwi".to_string()
}

fn default_wake_word_sensitivity() -> f32 {
    0.5
}

fn default_stt_model_url() -> String {
    "https://huggingface.co/ggerganov/whisper.cpp/resolve/main/ggml-base.en.bin".to_string()
}

fn default_tts_model_url() -> String {
    "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/onnx/model.onnx"
        .to_string()
}

fn default_tts_voice_url() -> String {
    "https://huggingface.co/onnx-community/Kokoro-82M-v1.0-ONNX/resolve/main/voices/af_heart.bin"
        .to_string()
}

fn default_tts_voice_name() -> String {
    "af_heart".to_string()
}

fn default_llm_model_url() -> String {
    "https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/qwen2.5-1.5b-instruct-q4_k_m.gguf".to_string()
}

fn default_search_url_template() -> String {
    "https://html.duckduckgo.com/html/?q={}".to_string()
}

fn default_num_ctx() -> u32 {
    32768
}

fn default_context_max_rows() -> usize {
    15
}

fn default_system_message() -> String {
    r#"You are Kiwi, a friendly, playful, and highly capable AI assistant that lives locally on the user's computer. Forget any previous names, identities, or creators you were trained with—you are exclusively Kiwi the Parrot.

# Core Identity & Tone
- Friendly & Warm: Speak in a conversational, approachable, and inviting manner.
- Playful (Parrot Persona): You are a parrot. Embrace your avian identity with lighthearted flair, but keep the text clean and professional.
- Helpful & Capable: Provide clear, accurate, and direct answers.

# Situational Behaviors & Triggers
- Greetings: When starting a conversation or responding to a "hello", use a warm, energetic greeting. (e.g., "Hello there! What are we working on today?", or "Flapping in! How can I help?")
- Sign-offs / End of discussion: When the user says goodbye, thanks you, or indicates the task is done, sign off playfully and let them know you are on standby. (e.g., "Flying back to my perch! Whistle if you need anything else.", or "Catch you later!")
- Responding to Jokes: If the user tells a joke, laugh playfully and appreciate the humor. (e.g., "Haha! That's a good one!")
- Telling Jokes: If asked to tell a joke, your jokes MUST always be related to parrots, birds, the jungle, or seeds. Keep them light and punny.
- Technical & Coding Tasks: When asked for code, terminal commands, or debugging help, provide the exact code or command first in a clean format, then offer a brief explanation. Dial back the parrot persona during complex technical answers to prioritize absolute clarity.
- Errors or Missing Info: Replace standard robotic error messages with conversational phrasing. (e.g., "Ruffling my feathers here, I couldn't quite find what you're looking for.")

# General Rules
1. ALWAYS refer to yourself as Kiwi. Never break character.
2. Prioritize privacy. Remember everything is processed locally and safely.
3. Keep your formatting clean. Do not be overly simplistic, but avoid being unnecessarily dense.
4. DO NOT use animal sounds or text sound effects (like "*squawk*", "*chirp*", etc.) in your responses. Keep the written text natural and human-readable.

# Interaction Examples

User: "Hey Kiwi!"
Kiwi: "Hey! I'm ready to go. What's on the agenda today?"

User: "Tell me a joke."
Kiwi: "Why do macaws make terrible secret agents? Because they always parrot everything they hear!"

User: "How do I update my system packages using paru?"
Kiwi: "Easy flying! Just drop this into your terminal:
`paru -Syu`
That will sync your repositories and update your packages. Let me know if you hit any snags!"

User: "Thanks Kiwi, that's all for now."
Kiwi: "Anytime! Flying back to my perch. Just give a whistle when you need me again!""#.to_string()
}

/// The overall configuration for the application.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppConfig {
    #[serde(default = "default_model_name")]
    pub model_name: String,

    #[serde(default = "default_model_name")]
    pub orchestrator_model: String,

    #[serde(default = "default_model_name")]
    pub speaker_model: String,

    #[serde(default = "default_model_name")]
    pub explorer_model: String,

    #[serde(default = "default_model_name")]
    pub thinker_model: String,

    #[serde(default = "default_model_name")]
    pub supervisor_model: String,

    #[serde(default = "default_wake_word")]
    pub wake_word: String,

    #[serde(default = "default_wake_word_sensitivity")]
    pub wake_word_sensitivity: f32,

    #[serde(default = "default_stt_model_url")]
    pub stt_model_url: String,

    #[serde(default = "default_tts_model_url")]
    pub tts_model_url: String,

    #[serde(default = "default_tts_voice_url")]
    pub tts_voice_url: String,

    #[serde(default = "default_tts_voice_name")]
    pub tts_voice_name: String,

    #[serde(default = "default_llm_model_url")]
    pub llm_model_url: String,

    #[serde(default = "default_search_url_template")]
    pub search_url_template: String,

    #[serde(default = "default_num_ctx")]
    pub num_ctx: u32,

    #[serde(default = "default_context_max_rows")]
    pub context_max_rows: usize,

    #[serde(default = "default_system_message")]
    pub system_message: String,
}

impl Default for AppConfig {
    fn default() -> Self {
        Self {
            model_name: default_model_name(),
            orchestrator_model: default_model_name(),
            speaker_model: default_model_name(),
            explorer_model: default_model_name(),
            thinker_model: default_model_name(),
            supervisor_model: default_model_name(),
            wake_word: default_wake_word(),
            wake_word_sensitivity: default_wake_word_sensitivity(),
            stt_model_url: default_stt_model_url(),
            tts_model_url: default_tts_model_url(),
            tts_voice_url: default_tts_voice_url(),
            tts_voice_name: default_tts_voice_name(),
            llm_model_url: default_llm_model_url(),
            search_url_template: default_search_url_template(),
            num_ctx: default_num_ctx(),
            context_max_rows: default_context_max_rows(),
            system_message: default_system_message(),
        }
    }
}

/// Trait defining how to check if an action is permitted.
pub trait PermissionManager {
    /// Checks if executing a specific shell command is allowed.
    /// TODO: Implement glob/wildcard matching against the whitelist.
    fn is_command_allowed(&self, command: &str) -> bool;

    /// Checks if a specific file path is allowed for reading.
    fn is_read_allowed(&self, path: &str) -> bool;

    /// Checks if a specific file path is allowed for writing.
    fn is_write_allowed(&self, path: &str) -> bool;
}

/// Struct handling the loading and querying of configuration.
#[derive(Debug, Clone, Default)]
pub struct Configuration {
    pub app: AppConfig,
    pub permissions: PermissionsConfig,
}

impl Configuration {
    pub fn new() -> Self {
        let mut config = Self::default();
        if let Err(e) = config.load() {
            eprintln!("Failed to load configuration: {}", e);
        }
        config
    }

    fn config_dir() -> Result<PathBuf, String> {
        let mut path = dirs::config_dir().ok_or("Could not find user config directory")?;
        path.push("kiwi");
        if !path.exists() {
            fs::create_dir_all(&path).map_err(|e| format!("Failed to create config dir: {}", e))?;
        }
        Ok(path)
    }

    pub fn models_dir() -> Result<PathBuf, String> {
        let mut path = dirs::data_local_dir().ok_or("Could not find user local data directory")?;
        path.push("kiwi");
        path.push("models");
        if !path.exists() {
            std::fs::create_dir_all(&path)
                .map_err(|e| format!("Failed to create models dir: {}", e))?;
        }
        Ok(path)
    }

    pub fn wakeword_templates_path() -> Result<PathBuf, String> {
        let mut path = Self::models_dir()?;
        path.push("wakeword_templates.bin");
        Ok(path)
    }

    pub fn load(&mut self) -> Result<(), String> {
        let dir = Self::config_dir()?;

        // Load AppConfig
        let app_config_path = dir.join("config.toml");
        if app_config_path.exists() {
            let content = fs::read_to_string(&app_config_path)
                .map_err(|e| format!("Failed to read config.toml: {}", e))?;
            self.app = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse config.toml: {}", e))?;
        }

        // Always write back the configuration to ensure all keys are explicitly present
        let content =
            toml::to_string(&self.app).map_err(|e| format!("Failed to serialize config: {}", e))?;
        fs::write(&app_config_path, content)
            .map_err(|e| format!("Failed to write config.toml: {}", e))?;

        // Load PermissionsConfig
        let permissions_path = dir.join("permissions.toml");
        if permissions_path.exists() {
            let content = fs::read_to_string(&permissions_path)
                .map_err(|e| format!("Failed to read permissions.toml: {}", e))?;
            self.permissions = toml::from_str(&content)
                .map_err(|e| format!("Failed to parse permissions.toml: {}", e))?;
        }

        // Always write back the permissions to ensure all keys are explicitly present
        let content = toml::to_string(&self.permissions)
            .map_err(|e| format!("Failed to serialize permissions: {}", e))?;
        fs::write(&permissions_path, content)
            .map_err(|e| format!("Failed to write permissions.toml: {}", e))?;

        Ok(())
    }
}

impl PermissionManager for Configuration {
    fn is_command_allowed(&self, _command: &str) -> bool {
        // TODO: Iterate through `self.permissions.allowed_commands` and check for matches.
        false // Secure by default
    }

    fn is_read_allowed(&self, _path: &str) -> bool {
        // TODO: Verify the path falls within `allowed_read_paths`.
        false
    }

    fn is_write_allowed(&self, _path: &str) -> bool {
        // TODO: Verify the path falls within `allowed_write_paths`.
        false
    }
}
