use axum::{
    extract::{Path, State},
    Json,
};

use crate::api::routes::AppState;
use crate::error::AppError;
use crate::services::threshold_ai_service::AnalyzeDeviceRequest;

pub async fn thresholds(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    match state.alert_service.thresholds(&device_id).await? {
        Some(t) => Ok(Json(serde_json::to_value(t)?)),
        None => Ok(Json(serde_json::json!({
            "device_id": device_id,
            "thresholds": []
        }))),
    }
}

pub async fn analyze_thresholds(
    State(state): State<AppState>,
    Json(request): Json<AnalyzeDeviceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .threshold_ai_service
        .analyze(request)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "device_id": result.device_id,
        "device_type_detected": result.device_type_detected,
        "thresholds_created": result.thresholds_created,
        "thresholds": result.thresholds,
        "ignored_metrics": result.ignored_metrics,
        "general_notes": result.general_notes,
        "ai_model": result.ai_model
    })))
}

pub async fn predictions(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let preds = state.alert_service.cached_predictions().await?;
    Ok(Json(serde_json::json!({ "predictions": preds, "count": preds.len() })))
}
