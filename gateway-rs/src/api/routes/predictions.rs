use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use serde_json::{json, Value};

use crate::api::routes::AppState;
use crate::services::influx_service::PredictionPoint;
use crate::Result;

#[derive(Debug, Deserialize)]
pub struct GetPredictionsQuery {
    pub metric_type: Option<String>,
    #[serde(default = "default_range")]
    pub range: String,
}

fn default_range() -> String {
    "7d".to_string()
}

#[derive(Debug, Deserialize)]
pub struct SavePredictionsRequest {
    pub node_id: String,
    pub metric_type: String,
    pub model_type: String,
    pub predictions: Vec<PredictionPointInput>,
}

#[derive(Debug, Deserialize)]
pub struct PredictionPointInput {
    pub timestamp: i64,
    pub value: f64,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub confidence: Option<f64>,
}

pub async fn save_predictions(
    State(state): State<AppState>,
    Json(request): Json<SavePredictionsRequest>,
) -> Result<Json<Value>> {
    let predictions: Vec<PredictionPoint> = request
        .predictions
        .into_iter()
        .map(|p| PredictionPoint {
            timestamp: p.timestamp,
            value: p.value,
            lower_bound: p.lower_bound,
            upper_bound: p.upper_bound,
            confidence: p.confidence,
        })
        .collect();

    let count = predictions.len();

    state
        .influx_service
        .save_predictions(
            &request.node_id,
            &request.metric_type,
            &request.model_type,
            &predictions,
        )
        .await?;

    Ok(Json(json!({
        "success": true,
        "node_id": request.node_id,
        "metric_type": request.metric_type,
        "model_type": request.model_type,
        "count": count
    })))
}

pub async fn node_predictions(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Query(query): Query<GetPredictionsQuery>,
) -> Result<Json<Value>> {
    let result = state
        .influx_service
        .predictions(Some(&node_id), query.metric_type.as_deref(), &query.range)
        .await?;

    Ok(Json(result))
}
