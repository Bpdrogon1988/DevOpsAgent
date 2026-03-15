use axum::{
    routing::{get, post},
    Router,
    response::IntoResponse,
    http::{HeaderMap, StatusCode},
    body::Bytes,
    extract::State,
};
use std::sync::Arc;
use crate::security;
use crate::executor;
use crate::AppState;

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    "Agent is healthy and running!"
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    
    // 1. Get the signature from the headers
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|val| val.to_str().ok())
        .ok_or((StatusCode::UNAUTHORIZED, "Missing signature header".to_string()))?;

    // 2. Validate the signature against the raw body bytes
    let payload = std::str::from_utf8(&body)
        .map_err(|_| ((StatusCode::BAD_REQUEST, "Invalid UTF-8 body".to_string())))?;

    if !security::verify_github_signature(payload, signature) {
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature".to_string()));
    }

    // 3. Security Passed! Trigger background workflow steps
    println!("Webhook authenticated! Triggering workflow: {}", state.workflow.name);
    
    let workflow_clone = state.workflow.clone();
    tokio::spawn(async move {
        for step in workflow_clone.steps {
            println!("==> Running step: {}", step.name);
            match executor::run_script(step.command.clone()).await {
                Ok(output) => println!("✅ Success ({}):\n{}", step.name, output),
                Err(err) => {
                    println!("❌ Failed ({}):\n{}", step.name, err);
                    println!("🤖 Asking AI to diagnose the error...");
                    
                    match crate::rag::triage_error(&err).await {
                        Ok(diagnosis) => println!("💡 AI Diagnosis:\n{}", diagnosis),
                        Err(rag_err) => println!("⚠️ Failed to get AI diagnosis: {}", rag_err),
                    }
                    
                    break; // Stop running steps if one fails
                }
            }
        }
    });

    Ok((StatusCode::ACCEPTED, "Webhook accepted and workflow queued"))
}

