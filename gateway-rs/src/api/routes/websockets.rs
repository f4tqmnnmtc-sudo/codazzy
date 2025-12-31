use crate::api::routes::AppState;
use crate::error::AppError;
use axum::{
    extract::{
        ws::{WebSocket, WebSocketUpgrade},
        State,
    },
    response::Response,
    Json,
};
use serde_json::{json, Value};

pub async fn upgrade(ws: WebSocketUpgrade, State(state): State<AppState>) -> Response {
    ws.on_upgrade(move |socket| serve(socket, state))
}

async fn serve(socket: WebSocket, state: AppState) {
    state
        .websocket_service
        .clone()
        .accept(socket)
        .await;
}

pub async fn ws_stats(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let stats = state.websocket_service.stats();
    Ok(Json(json!({
        "active_connections": stats.active_connections,
        "topics": stats.topics,
        "total_messages": stats.total_messages
    })))
}
