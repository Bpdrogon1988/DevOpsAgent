mod api;
mod config;
mod executor;
mod rag;
mod security;

use axum::Router;
use std::sync::Arc;
use std::net::SocketAddr;

pub struct AppState {
    pub workflow: config::Workflow,
}

#[tokio::main]
async fn main() {
    println!("Starting DevOps Agent...");

    // 1. Load configuration (secrets, workflows)
    let workflow = config::load_config();
    let state = Arc::new(AppState { workflow });

    // 2. Setup the application router with shared state
    let app = Router::new()
        .merge(api::routes(state.clone()));

    // 3. Start the server
    let addr = SocketAddr::from(([127, 0, 0, 1], 3000));
    println!("Listening on {}", addr);
    
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
