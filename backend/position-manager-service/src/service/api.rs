use std::net::SocketAddr;
use std::sync::Arc;

use anyhow::Result;
use axum::{
    extract::Path, extract::State, routing::delete, routing::get, routing::post, routing::put,
    Json, Router,
};
use serde_json::json;

use super::manager::PositionManager;
use super::models::{ClosePositionRequest, ModifyPositionRequest, OpenPositionRequest};
use super::repositories::{InMemoryPositionRepository, PositionRepository};
use super::risk::default_leverage_tiers;

#[derive(Clone)]
pub struct AppState {
    manager: PositionManager,
}

pub async fn build_router() -> Result<(
    SocketAddr,
    impl std::future::Future<Output = hyper::Result<()>> + Send,
)> {
    let repository: Arc<dyn PositionRepository> = Arc::new(InMemoryPositionRepository::default());
    let manager = PositionManager::new(repository, default_leverage_tiers());
    let state = AppState { manager };

    let app = Router::new()
        .route("/positions/open", post(open_position))
        .route("/positions/:id/modify", put(modify_position))
        .route("/positions/:id/close", delete(close_position))
        .route("/positions/:id", get(get_position))
        .route("/users/:id/positions", get(user_positions))
        .with_state(state);

    let addr = SocketAddr::from(([0, 0, 0, 0], 8080));
    let server = axum::Server::bind(&addr).serve(app.into_make_service());
    Ok((addr, server))
}

async fn open_position(
    State(state): State<AppState>,
    Json(request): Json<OpenPositionRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    state
        .manager
        .open_position(request)
        .await
        .map(|position| Json(json!({ "position": position })))
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)
}

async fn modify_position(
    State(state): State<AppState>,
    Path(position_id): Path<u64>,
    Json(mut request): Json<ModifyPositionRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    request.position_id = position_id;
    state
        .manager
        .modify_position(request)
        .await
        .map(|position| Json(json!({ "position": position })))
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)
}

async fn close_position(
    State(state): State<AppState>,
    Path(position_id): Path<u64>,
    Json(mut request): Json<ClosePositionRequest>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    request.position_id = position_id;
    state
        .manager
        .close_position(request)
        .await
        .map(|_| Json(json!({ "status": "closed" })))
        .map_err(|_| axum::http::StatusCode::BAD_REQUEST)
}

async fn get_position(
    State(state): State<AppState>,
    Path(position_id): Path<u64>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    state
        .manager
        .get_position(position_id)
        .await
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
        .and_then(|maybe_position| {
            maybe_position
                .map(|position| Json(json!({ "position": position })))
                .ok_or(axum::http::StatusCode::NOT_FOUND)
        })
}

async fn user_positions(
    State(state): State<AppState>,
    Path(owner): Path<String>,
) -> Result<Json<serde_json::Value>, axum::http::StatusCode> {
    state
        .manager
        .positions_for_owner(&owner)
        .await
        .map(|positions| Json(json!({ "owner": owner, "positions": positions })))
        .map_err(|_| axum::http::StatusCode::INTERNAL_SERVER_ERROR)
}
