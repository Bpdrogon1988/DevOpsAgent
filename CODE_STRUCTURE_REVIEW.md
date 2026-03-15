# Code Structure Review

## High-level layout

- `src/main.rs` is a thin composition root that wires configuration, shared state, routes, and the HTTP server.
- `src/api.rs` contains route declarations and webhook request handling.
- `src/security.rs` encapsulates GitHub webhook signature validation logic.
- `src/executor.rs` executes workflow shell commands asynchronously via `spawn_blocking`.
- `src/config.rs` loads environment variables and parses `workflow.yaml` into typed structs.
- `src/rag.rs` calls OpenAI chat completions for error triage.

This is a clean, modular layout for a small service with clear module boundaries.

## Structure quality assessment

### What is working well

1. **Separation of concerns**
   - API, security, config parsing, command execution, and AI triage are each isolated in separate modules.
2. **Thin entrypoint**
   - `main.rs` mostly performs wiring and startup, which keeps bootstrapping easy to follow.
3. **Typed workflow model**
   - `Workflow` and `Step` structs provide a simple, maintainable contract around `workflow.yaml`.
4. **Basic test coverage in a critical area**
   - Signature validation has unit tests for valid, invalid, and malformed signature formats.

### Structural risks / improvement opportunities

1. **Configuration boundary is mixed**
   - `load_config()` currently loads env vars and parses workflow in a single function. Splitting environment/config concerns into explicit config structs would improve testability.
2. **Error handling strategy is mostly panic-based at startup**
   - Multiple `expect`/`unwrap` calls are used for startup paths. Returning richer errors from initialization can improve operability.
3. **API handler is doing orchestration + business logic**
   - `handle_webhook` currently verifies security, schedules workflow execution, and invokes AI triage on failures. Extracting a `workflow_service` layer would reduce handler complexity.
4. **Cross-cutting logging is print-based**
   - `println!` is spread across modules. Introducing structured logging (`tracing`) would help observability and maintainability.
5. **Security defaults may be unsafe for production**
   - Falling back to `default_secret` is convenient for local development but risky in deployed environments.

## Suggested target package structure (next iteration)

```text
src/
  main.rs
  app/
    mod.rs            # app wiring/bootstrap
    state.rs          # AppState
  http/
    mod.rs
    routes.rs
    handlers.rs       # transport-only logic
  services/
    mod.rs
    workflow.rs       # execute workflow + failure policy
    triage.rs         # AI diagnosis orchestration
  domain/
    mod.rs
    workflow.rs       # Workflow/Step types and invariants
  infra/
    mod.rs
    config.rs
    shell.rs
    github_signature.rs
    ai_client.rs
```

## Priority recommendations

1. Add a small service layer (`services/workflow.rs`) and move execution/triage orchestration out of `api.rs`.
2. Replace `println!` with `tracing` + `tracing-subscriber` and include per-request context.
3. Introduce explicit runtime config struct (secret, bind address, model name) and validate it at startup.
4. Remove insecure secret fallback in non-development environments.
5. Add unit tests around config parsing and workflow execution branching (success/failure stop behavior).
