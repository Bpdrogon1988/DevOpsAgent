# 🚀 Rust DevOps Agent

A secure, high-performance DevOps automation agent built in Rust. This agent listens for GitHub webhooks, executes dynamic workflows, and uses AI-powered RAG (Retrieval-Augmented Generation) to autonomously diagnose and triage deployment failures.

## ✨ Features

- **🔒 Secure Webhook Listener**: HMAC-SHA256 signature verification for all incoming GitHub events.
- **⚙️ Dynamic Workflow Engine**: Define deployment steps in a simple `workflow.yaml` file—no recompilation required.
- **🤖 AI-Powered SRE**: Integrated with OpenAI (GPT-5.4) to provide instant, human-readable diagnoses for failed shell commands.
- **⚡ High Performance**: Built on the `axum` web framework and `tokio` asynchronous runtime.
- **📦 Clean Architecture**: Modular Rust design for easy extensibility.

## 🛠️ Architecture

The agent consists of several core modules:
- `api.rs`: Handles HTTP routes and Shared State.
- `security.rs`: Cryptographic validation of webhook signatures.
- `executor.rs`: Asynchronous shell command execution.
- `config.rs`: YAML parsing and Environment Variable management.
- `rag.rs`: AI Triage engine for error analysis.

## 🚀 Getting Started

### 1. Configuration
Create a `.env` file in the root directory:
```env
GITHUB_WEBHOOK_SECRET=your_secret_here
OPENAI_API_KEY=your_openai_key_here
QDRANT_URL=http://localhost:6333
```

### 2. Define Your Workflow
Edit `workflow.yaml` to set your deployment steps:
```yaml
name: "Production Deployment"
steps:
  - name: "Pull Code"
    command: "git pull origin main"
  - name: "Build"
    command: "cargo build --release"
  - name: "Restart Service"
    command: "systemctl restart my_app"
```

### 3. Run the Agent
```bash
cargo run
```
The agent will start listening on `http://localhost:3000`.

## 🛡️ Security
This agent implements strict push protection. Secrets are managed via environment variables and are excluded from version control by default.

---
Built with ❤️ using Rust.
