# Kiwi Implementation Plan 🦜

This document outlines the independent tasks required to implement the Kiwi local AI desktop assistant. These high-level tasks are designed to be implemented in parallel by different agents or developers.

## Task 1: Core Daemon & Wake Word Engine
**Goal:** Establish the background daemon process and the wake word detection system.
- **Description:** Implement the main Rust daemon that runs in the background. Integrate an audio processing library to listen for the "Hey Kiwi" wake word.
- **Key Components:** Audio stream capture, Wake Word Engine, basic event loop (Tokio).
- **Output:** A daemon that can successfully log or emit an event when "Hey Kiwi" is spoken.

## Task 2: Speech-to-Text (STT) & Text-to-Speech (TTS) Pipelines
**Goal:** Handle the conversion between voice and text.
- **Description:** Implement the STT engine to convert the user's spoken prompt into text after the wake word is triggered. Implement the TTS engine with the playful parrot voice persona to vocalize the assistant's responses.
- **Key Components:** STT Engine, TTS Engine, audio playback.
- **Dependencies:** Basic audio stream handling from Task 1 (for STT input).

## Task 3: Local LLM Integration & Context Management
**Goal:** Integrate the local language model and manage conversation context.
- **Description:** Set up the interface to load and query the local LLM (1B - 9B parameters, e.g., via GGUF formats). Implement a context window manager to keep track of the current conversation state and system prompts (including the parrot persona instructions).
- **Key Components:** LLM Interface, Context Window, Intent Router (basic routing logic).
- **Output:** A module that takes text input, routes it, queries the LLM, and returns text output.

## Task 4: Web Search & Scraping Tool
**Goal:** Enable Kiwi to find real-time information on the web.
- **Description:** Build a tool that can be triggered by the Intent Router to perform Google searches and scrape website content. The results must be formatted to be injected into the LLM's context window.
- **Key Components:** Web Scraper, Search API integration (or fallback scraping).
- **Dependencies:** Can be developed entirely independently as a standalone library/module.

## Task 5: Plugin Engine (Rhai)
**Goal:** Implement the extensible plugin system using the Rhai scripting language.
- **Description:** Create the embedded Rhai environment. Define the API surface that plugins can use to interact with Kiwi's core (e.g., adding new commands or intents). Build a mechanism to load `.rhai` scripts from `~/.config/kiwi/plugins/`.
- **Key Components:** Rhai Engine initialization, Plugin Loader, Rust-to-Rhai bindings.
- **Output:** A working plugin manager that can execute custom Rhai scripts.

## Task 6: Permission Manager & System Execution
**Goal:** Safely manage permissions and execute system commands/apps.
- **Description:** Implement the Permission Manager that reads and enforces rules from `~/.config/kiwi/permissions.toml`. Build the execution module that runs shell commands or opens applications only if explicitly permitted by the configuration.
- **Key Components:** TOML Parser, Permission Manager, Command Executor.
- **Output:** A secure module that accepts execution requests and either runs them or returns a "Permission Denied" response based on the config.

## Task 7: Graphical Mascot UI (egui/eframe)
**Goal:** Build the playful parrot graphical popup.
- **Description:** Using `egui` and `eframe`, create an unobtrusive desktop UI that pops up when Kiwi is listening or speaking. Implement subtle animations (e.g., thinking, talking) and ensure the UI matches the brand guidelines (vibrant colors, clean typography).
- **Key Components:** eframe App, UI rendering, Animation logic, IPC/Event listener to trigger the popup.
- **Output:** A standalone window application that reacts to state changes (listening, thinking, speaking).