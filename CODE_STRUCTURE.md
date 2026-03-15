# Code Structure Review

## High-level layout

- `src/main.rs`: application entrypoint, startup orchestration, shared application state.
- `src/api.rs`: HTTP routing and webhook endpoint handling.
- `src/config.rs`: workflow YAML deserialization and environment bootstrapping.
- `src/security.rs`: GitHub webhook signature verification and unit tests.
- `src/executor.rs`: shell command execution in a blocking worker.
- `src/rag.rs`: OpenAI request construction and error triage response extraction.
- `workflow.yaml`: runtime workflow definition consumed by `config::load_config()`.

## Current architecture strengths

1. **Clear module boundaries**: transport (`api`), business flow (`executor` + `rag`), and support concerns (`config`, `security`) are split cleanly.
2. **Simple state model**: immutable `Workflow` stored in `Arc<AppState>` keeps concurrent request handling straightforward.
3. **Fast-fail security gate**: signature validation happens before any workflow execution.
4. **Background execution model**: webhook endpoint returns quickly (`202 ACCEPTED`) while steps run asynchronously.

## Structure risks and improvement opportunities

1. **Large handler responsibility in `handle_webhook`**
   - It currently validates headers, parses payload, verifies signatures, schedules jobs, handles per-step logging, and AI triage fallback.
   - Consider extracting:
     - `extract_signature(&HeaderMap) -> Result<&str, ...>`
     - `authenticate_webhook(payload, signature) -> Result<(), ...>`
     - `run_workflow(workflow: Workflow)` as a service function.

2. **Implicit dependency on process working directory**
   - `config::load_config()` reads `workflow.yaml` via relative path.
   - Consider reading from an explicit env var (`WORKFLOW_PATH`) with fallback for better deployment predictability.

3. **Execution and triage coupling**
   - `api.rs` directly invokes `rag::triage_error` on executor failure.
   - Introduce a small orchestration layer (e.g., `service.rs`) to keep HTTP concerns separate from workflow/triage behavior.

4. **Potentially unsafe default secret**
   - `security.rs` defaults `GITHUB_WEBHOOK_SECRET` to `"default_secret"`.
   - Structure is cleaner and safer if startup validates required secrets once and fails hard if absent.

5. **Observability is print-based**
   - Cross-module `println!` calls make production debugging harder.
   - Add structured logging (`tracing`) and correlation fields (workflow name, step name, request id).

## Suggested next refactor sequence

1. Add `service/workflow_runner.rs` and move background loop there.
2. Reduce `handle_webhook` to input validation + service call.
3. Add startup config validation for required secrets and workflow path.
4. Introduce `tracing` spans across request -> step -> AI triage chain.

