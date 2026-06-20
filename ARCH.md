# Kiwi Architecture 🦜

This document outlines the high-level architecture and key design decisions for Kiwi, the local AI desktop assistant.

## High-Level Architecture

Kiwi operates entirely locally, utilizing a modular pipeline for audio processing, language modeling, and execution. Below is a Mermaid.js diagram illustrating the core flow of data through the system.

```mermaid
graph TD
    %% Input Layer
    Mic([Microphone Input]) --> WW[Wake Word Engine]

    %% Processing Pipeline
    WW -- "'Hey Kiwi' detected" --> STT[Speech-to-Text Engine]
    STT --> Orchestrator

    %% Core Intelligence (Multi-Agent System)
    Orchestrator <--> Context(Context Window)
    Orchestrator <--> Thinker[Thinker: Intent & Logic]
    Orchestrator <--> Explorer[Explorer: Web Search & Recap]
    Orchestrator <--> Supervisor[Supervisor: Memory & Filtering]
    Orchestrator --> SpeakerAgent[Speaker: Final Response Generation]

    Thinker --> LLM_Think[Thinker LLM]
    Explorer --> LLM_Expl[Explorer LLM]
    Supervisor --> LLM_Sup[Supervisor LLM]
    SpeakerAgent --> LLM_Speak[Speaker LLM]

    %% Context & Tools
    Explorer -- "Requires info" --> Search[Google Search & Web Scraper]
    Search --> Context

    %% Execution & Extensibility
    Orchestrator --> Exec{Action Required?}
    Exec -- "Yes" --> Perm[Permission Manager]
    Perm -- "Allowed via config" --> Command[System Command/App Exec]
    Perm -- "Denied" --> DeniedMsg[Permission Denied Response]

    Exec -- "No" --> TTS[Text-to-Speech Engine]
    SpeakerAgent --> TTS
    Command --> TTS
    DeniedMsg --> TTS

    %% Output Layer
    TTS --> Speaker([Speaker Output])

    %% GUI & Plugins
    WW -. "Triggers" .-> GUI[Graphical Mascot Popup]
    Orchestrator -. "Loads" .-> Plugins[Rhai Plugin Engine]
    Plugins -. "Extends" .-> Orchestrator
```

## Key Design Decisions

### 1. Multi-Agent Architecture
**Why Multiple Agents?**
Kiwi is composed of several distinct agents, each responsible for a specific cognitive task. This allows for greater modularity and the ability to configure different LLMs (with varying sizes and capabilities) for different tasks:
- **Thinker:** Determines user intent and answers logical questions (e.g., "Do I need to search the web for this?").
- **Explorer:** Manages web searches and summarizes external information.
- **Supervisor:** Manages the memory bank, extracting keywords and determining context relevance.
- **Speaker:** Generates the final, persona-driven response presented to the user.
- **Orchestrator:** The central hub that coordinates the agents and processes user input into the final output. Agents do not communicate directly with each other; they only interface through the Orchestrator.

### 2. Language: Rust
**Why Rust?**
Kiwi is designed to be a lightweight background daemon. Rust provides the perfect balance of low-level control (necessary for audio stream processing and system interactions), high performance (crucial for running LLMs efficiently on consumer hardware), and memory safety.

### 3. Plugin System: Rhai
**Why Rhai?**
We want developers to easily extend Kiwi's capabilities without having to recompile the entire core application. [Rhai](https://rhai.rs/) is an embedded scripting language specifically designed for Rust. It is fast, safe, and easily binds to Rust functions, making it the ideal choice for user-defined scripts and plugins.

### 4. Permission Management
**Configuration-driven Security**
Because Kiwi runs locally and has the potential to execute system commands, security is paramount. Kiwi uses a strict, whitelist-based configuration file (`permissions.toml`). The assistant cannot execute any shell commands, open apps, or read/write files unless explicitly permitted by the user in this configuration file (e.g., allowing specific wildcard patterns like `git *`).

### 5. Local-First AI
**Privacy & Independence**
The core intelligence relies on small, quantized LLMs (ranging from 1B to 9B parameters) running entirely on the user's machine. This guarantees that private conversations are never sent to external servers, while still providing robust natural language understanding.

### 6. Web Search Integration
**Bridging the Knowledge Gap**
While local LLMs are great, their knowledge cutoff limits their ability to answer queries about current events. Kiwi is designed to seamlessly fall back to Google Search and web scraping when the LLM determines that real-time information is required to satisfy the user's prompt.