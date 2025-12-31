use crate::api::routes::AppState;
use crate::error::AppError;
use crate::models::teleco_device::AddDeviceRequest;
use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::{json, Value};

pub async fn list_devices(State(state): State<AppState>) -> Result<Json<Value>, AppError> {
    let devices = state.teleco_service.all_devices();
    Ok(Json(json!({ "devices": devices, "total": devices.len() })))
}

pub async fn add_device(
    State(state): State<AppState>,
    Json(request): Json<AddDeviceRequest>,
) -> Result<Json<Value>, AppError> {
    let device = state.teleco_service.add_device(request).await?;
    Ok(Json(json!({ "device": device })))
}

pub async fn remove_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<Value>, AppError> {
    if state.teleco_service.remove_device(&device_id).await? {
        Ok(Json(json!({ "device_id": device_id })))
    } else {
        Err(AppError::NotFound(format!(
            "Dispositivo no encontrado: {}",
            device_id
        )))
    }
}
