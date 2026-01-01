use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use serde_json::{json, Value};
use std::time::Duration;

use crate::api::routes::AppState;
use crate::config::CONFIG;
use crate::error::AppError;
use crate::services::influx_service::PredictionPoint;
use crate::Result;

#[derive(Debug, Deserialize)]
pub struct GetPredictionsQuery {
    pub metric_type: Option<String>,
    #[serde(default = "default_range")]
    pub range: String,
}

fn default_range() -> String { "7d".into() }

#[derive(Debug, Deserialize)]
pub struct SavePredictionsRequest {
    pub node_id: String,
    pub metric_type: String,
    pub model_type: String,
    pub predictions: Vec<PredictionInput>,
}

#[derive(Debug, Deserialize)]
pub struct PredictionInput {
    pub timestamp: i64,
    pub value: f64,
    pub lower_bound: Option<f64>,
    pub upper_bound: Option<f64>,
    pub confidence: Option<f64>,
}

pub async fn save_predictions(
    State(st): State<AppState>,
    Json(req): Json<SavePredictionsRequest>,
) -> Result<Json<Value>> {
    let pts: Vec<PredictionPoint> = req.predictions.into_iter()
        .map(|p| PredictionPoint {
            timestamp: p.timestamp, value: p.value,
            lower_bound: p.lower_bound, upper_bound: p.upper_bound,
            confidence: p.confidence,
        })
        .collect();

    let n = pts.len();
    st.influx_service.save_predictions(&req.node_id, &req.metric_type, &req.model_type, &pts).await?;

    Ok(Json(json!({
        "success": true,
        "node_id": req.node_id,
        "metric_type": req.metric_type,
        "model_type": req.model_type,
        "count": n
    })))
}

pub async fn node_predictions(
    State(st): State<AppState>,
    Path(node_id): Path<String>,
    Query(q): Query<GetPredictionsQuery>,
) -> Result<Json<Value>> {
    let out = st.influx_service
        .predictions(Some(&node_id), q.metric_type.as_deref(), &q.range)
        .await?;
    Ok(Json(out))
}

pub async fn forecast_daily(Json(req): Json<Value>) -> Result<Json<Value>> {
    call_profeta("daily", req).await
}

pub async fn forecast_weekly(Json(req): Json<Value>) -> Result<Json<Value>> {
    call_profeta("weekly", req).await
}

async fn call_profeta(period: &str, payload: Value) -> Result<Json<Value>> {
    let cli = reqwest::Client::builder()
        .timeout(Duration::from_secs(180))
        .build()
        .map_err(|e| AppError::Internal(format!("Error creando cliente HTTP: {e}")))?;

    let url = format!("{}/metrics/forecast/{}", CONFIG.profeta_url, period);

    let resp = cli.post(&url).json(&payload).send().await
        .map_err(|e| AppError::Internal(format!("Error en peticion a Profeta: {e}")))?;

    if !resp.status().is_success() {
        let (code, body) = (resp.status(), resp.text().await.unwrap_or_default());
        return Err(AppError::Internal(format!("Profeta fallo ({code}): {body}")));
    }

    let data: Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Respuesta invalida de Profeta: {e}")))?;

    Ok(Json(data))
}
