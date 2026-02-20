use axum::{
    routing::{get, post},
    Router,
};

use crate::{handlers, state::AppState};

/// Contract-related routes
pub fn contract_routes() -> Router<AppState> {
    let contracts_nested = Router::new()
        .route(
            "/",
            get(handlers::list_contracts).post(handlers::publish_contract),
        )
        .route("/graph", get(handlers::get_contract_graph))
        .route("/verify", post(handlers::verify_contract))
        .route("/{id}", get(handlers::get_contract))
        .route("/{id}/versions", get(handlers::get_contract_versions));

    Router::new().nest("/api/contracts", contracts_nested)
}

/// Publisher-related routes
pub fn publisher_routes() -> Router<AppState> {
    let publishers_nested = Router::new()
        .route("/", post(handlers::create_publisher))
        .route("/{id}", get(handlers::get_publisher))
        .route("/{id}/contracts", get(handlers::get_publisher_contracts));

    Router::new().nest("/api/publishers", publishers_nested)
}

/// Health check routes
pub fn health_routes() -> Router<AppState> {
    Router::new()
        .route("/health", get(handlers::health_check))
        .route("/api/stats", get(handlers::get_stats))
}
