use std::sync::Arc;

use axum::Json;
use axum::{
    Router,
    body::Bytes,
    extract::State,
    http::{HeaderMap, StatusCode},
    response::IntoResponse,
    routing::{get, post},
};
use tracing::{Instrument, info, info_span, warn};

use crate::{AppState, security};

pub fn routes(state: Arc<AppState>) -> Router {
    Router::new()
        .route("/health", get(health_check))
        .route("/capabilities", get(capabilities))
        .route("/webhook", post(handle_webhook))
        .with_state(state)
}

async fn health_check() -> impl IntoResponse {
    "Agent is healthy and running!"
}

async fn capabilities(State(state): State<Arc<AppState>>) -> impl IntoResponse {
    Json(state.workflow_service.capabilities())
}

async fn handle_webhook(
    State(state): State<Arc<AppState>>,
    headers: HeaderMap,
    body: Bytes,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let signature = headers
        .get("x-hub-signature-256")
        .and_then(|val| val.to_str().ok())
        .ok_or((
            StatusCode::UNAUTHORIZED,
            "Missing signature header".to_string(),
        ))?;

    let delivery_id = headers
        .get("x-github-delivery")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("unknown");

    let event_name = headers
        .get("x-github-event")
        .and_then(|val| val.to_str().ok())
        .unwrap_or("unknown");

    let span = info_span!("webhook_request", delivery_id = %delivery_id, event = %event_name);

    if !security::verify_github_signature(&body, signature, &state.runtime.github_webhook_secret) {
        warn!(delivery_id = %delivery_id, "Invalid webhook signature");
        return Err((StatusCode::UNAUTHORIZED, "Invalid signature".to_string()));
    }

    info!(delivery_id = %delivery_id, workflow = %state.workflow.name, "Webhook authenticated; queueing workflow");

    let workflow_clone = state.workflow.clone();
    let service = state.workflow_service.clone();

    tokio::spawn(
        async move {
            let report = service.run_workflow(workflow_clone).await;
            info!(
                workflow = %report.workflow_name,
                halted_on_failure = report.halted_on_failure,
                steps_run = report.steps.len(),
                "Workflow execution completed"
            );
        }
        .instrument(span),
    );

    Ok((StatusCode::ACCEPTED, "Webhook accepted and workflow queued"))
}
