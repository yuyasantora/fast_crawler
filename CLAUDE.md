# Project: My Legal Engine (Rust + Typst)

This project is a **Rust-based** web intelligence engine designed to collect, structure, and visualize legal information using **Typst** for generating high-quality PDF reports.
The initial use case focuses on fetching Japanese patent judgments from "IP Force," summarizing them using LLMs (Local/Cloud), and generating reports featuring "Claim Charts."

## 1. Tech Stack & Dependencies

* **Language**: Rust (Edition 2021)
* **Core Logic**:
    * `tokio`: Async runtime
    * `reqwest`: HTTP client
    * `scraper`: HTML parsing
    * `anyhow`: Error handling
    * `serde`, `serde_json`: JSON serialization/deserialization
    * `async_trait`: Async traits support
* **Template Engine**:
    * `askama`: Type-safe binding of Rust structs to Typst templates at compile time
* **Document Generation**:
    * `typst`: Typesetting system (executed via CLI)
* **LLM Backend** (Pluggable):
    * `ollama-rs` (For initial development): Local LLM (Qwen/Llama3)
    * `candle` (For future distribution): Rust native inference
    * `async-openai` (Fallback/Cloud)

## 2. Architecture: "The Intelligence Engine"

The architecture separates the **Generic Engine** from **Site-Specific Cartridges (Plugins)**.

### 2.1 Directory Structure

```text
my_legal_engine/
├── Cargo.toml
├── prompts/              # System prompts for LLMs (Loaded at runtime)
│   └── ip_force.md       # Instructions for Patent Judgments
├── templates/            # Askama/Typst templates
│   └── patent_report.typ
├── src/
│   ├── main.rs           # Entry point & Main loop
│   ├── traits.rs         # WebResource trait definition (Abstraction Layer)
│   ├── llm.rs            # LLM abstraction (Ollama/Candle/OpenAI)
│   └── plugins/          # Site-specific logic (Cartridges)
│       ├── mod.rs
│       └── ip_force.rs   # IP Force Patent Cartridge


