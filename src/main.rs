mod api;
mod config;
mod executor;
mod rag;
mod security;
mod services;

use std::sync::Arc;

use axum::Router;
use config::{RuntimeConfig, Workflow};
use services::workflow::WorkflowService;
use tracing::info;

pub struct AppState {
    pub workflow: Workflow,
    pub runtime: RuntimeConfig,
    pub workflow_service: Arc<WorkflowService>,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_target(true)
        .with_level(true)
        .compact()
        .init();

    info!("Starting DevOps Agent...");

    let app_config =
        config::load_config().unwrap_or_else(|err| panic!("failed to load configuration: {err}"));
    let workflow_service = Arc::new(WorkflowService::new(
        app_config.runtime.openai_model.clone(),
    ));

    let state = Arc::new(AppState {
        workflow: app_config.workflow,
        runtime: app_config.runtime.clone(),
        workflow_service,
    });

    let app = Router::new().merge(api::routes(state));

    info!(address = %app_config.runtime.bind_addr, "Listening");
    let listener = tokio::net::TcpListener::bind(app_config.runtime.bind_addr)
        .await
        .unwrap();
    axum::serve(listener, app).await.unwrap();
}
