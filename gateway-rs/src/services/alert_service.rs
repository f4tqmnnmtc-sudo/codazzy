use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::models::alerts::*;
use crate::services::InfluxService;
use chrono::Utc;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::{collections::HashMap, sync::Arc};
use tokio::time::{interval, Duration};
use tracing::{debug, error, info, warn};

pub struct AlertService {
    pool: PgPool,
    influx: Option<Arc<InfluxService>>,
    http: reqwest::Client,
}

impl AlertService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool, influx: None, http: reqwest::Client::new() }
    }

    pub fn with_influx(pool: PgPool, influx: Arc<InfluxService>) -> Self {
        Self { pool, influx: Some(influx), http: reqwest::Client::new() }
    }

    pub async fn thresholds(&self, device_id: &str) -> Result<Option<DeviceThresholds>> {
        let rows = sqlx::query(
            "SELECT metric_name,warning_threshold,critical_threshold,comparison,duration_seconds,created_at,updated_at \
             FROM alert_thresholds WHERE device_id=$1"
        ).bind(device_id).fetch_all(&self.pool).await.map_err(AppError::Database)?;

        if rows.is_empty() { return Ok(None) }

        let (mut ca, mut ua) = (Utc::now(), Utc::now());
        let ths = rows.iter().map(|r| {
            ca = r.get("created_at"); ua = r.get("updated_at");
            ThresholdConfig {
                metric_name: r.get("metric_name"),
                warning_threshold: r.get("warning_threshold"),
                critical_threshold: r.get("critical_threshold"),
                comparison: r.get::<Option<String>,_>("comparison").unwrap_or_default(),
                duration_seconds: r.get::<Option<i32>,_>("duration_seconds").map(|d| d as u32),
            }
        }).collect();

        Ok(Some(DeviceThresholds { device_id: device_id.into(), thresholds: ths, created_at: ca, updated_at: ua }))
    }

    pub async fn all_thresholds(&self) -> Result<Vec<DeviceThresholdsAI>> {
        let rows = sqlx::query(
            "SELECT device_id,metric_name,warning_threshold,critical_threshold,comparison,created_at,updated_at \
             FROM alert_thresholds ORDER BY device_id,metric_name"
        ).fetch_all(&self.pool).await.map_err(AppError::Database)?;

        let mut m: HashMap<String, DeviceThresholdsAI> = HashMap::with_capacity(rows.len() / 3);
        for r in rows {
            let did: String = r.get("device_id");
            let metric: String = r.get("metric_name");
            m.entry(did.clone()).or_insert_with(|| DeviceThresholdsAI {
                device_id: did.clone(), device_name: did.clone(), device_type: "server".into(),
                thresholds: vec![], created_at: r.get("created_at"), updated_at: r.get("updated_at"),
            }).thresholds.push(ThresholdConfigAI {
                metric_name: metric.clone(),
                display_name: Some(metric.replace('_', " ")),
                unit: Some("%".into()),
                warning_threshold: r.get("warning_threshold"),
                critical_threshold: r.get("critical_threshold"),
                comparison: r.get::<Option<String>,_>("comparison").unwrap_or_else(|| "gt".into()),
                priority: "medium".into(), reasoning: None, ai_model: None, enabled: true,
            });
        }
        Ok(m.into_values().collect())
    }

    pub async fn cached_predictions(&self) -> Result<Vec<CachedPrediction>> {
        let rows = sqlx::query(
            "SELECT DISTINCT ON(device_id,metric_name) \
             id,device_id,device_name,metric_name,display_name,current_value,predicted_value,\
             threshold_value,threshold_type,predicted_at,confidence,trend,hours_until,created_at \
             FROM prediction_cache WHERE hours_until>=0 ORDER BY device_id,metric_name,created_at DESC"
        ).fetch_all(&self.pool).await.map_err(AppError::Database)?;

        Ok(rows.iter().map(|r| CachedPrediction {
            id: r.get("id"), device_id: r.get("device_id"), device_name: r.get("device_name"),
            metric_name: r.get("metric_name"), display_name: r.get("display_name"),
            current_value: r.get("current_value"), predicted_value: r.get("predicted_value"),
            threshold_value: r.get("threshold_value"), threshold_type: r.get("threshold_type"),
            predicted_at: r.get("predicted_at"), confidence: r.get("confidence"),
            trend: r.get("trend"), hours_until: r.get("hours_until"), created_at: r.get("created_at"),
        }).collect())
    }

    pub async fn start_scheduler(self: Arc<Self>) {
        if !CONFIG.prediction_enabled { info!("prediction scheduler disabled"); return }
        let secs = CONFIG.prediction_interval_secs;
        info!("prediction scheduler: every {secs}s");
        tokio::spawn(async move {
            let mut tick = interval(Duration::from_secs(secs));
            loop { tick.tick().await; if let Err(e) = self.run_predictions().await { error!("prediction cycle: {e}") } }
        });
    }

    async fn run_predictions(&self) -> Result<()> {
        let influx = self.influx.as_ref().ok_or_else(|| AppError::Internal("no influx".into()))?;
        let devs = self.all_thresholds().await?;
        if devs.is_empty() { return Ok(()) }
        debug!("predictions for {} devices", devs.len());

        let mut buf: Vec<CachedPrediction> = Vec::with_capacity(devs.len() * 3);

        // Mapeo de nombres de metricas a componentes de InfluxDB
        let resolve_metric = |name: &str| -> Option<&'static str> {
            let lc = name.to_lowercase();
            if lc.contains("cpu") { Some("cpu") }
            else if lc.contains("mem") { Some("memory_percent") }
            else if lc.contains("disk") { Some("fs_root_usage_percent") }
            else { None }
        };

        for dev in &devs {
            for th in dev.thresholds.iter().filter(|t| t.enabled) {
                let Some(metric) = resolve_metric(&th.metric_name) else { continue };

                let mut pts = match influx.metric_history(&dev.device_id, metric, "24h").await {
                    Ok(v) if v.len() >= 10 => v,
                    Ok(v) => { debug!("insufficient {}/{}: {} pts", dev.device_id, metric, v.len()); continue }
                    Err(e) => { debug!("no hist {}/{}: {e}", dev.device_id, metric); continue }
                };
                pts.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));
                pts.dedup_by(|a, b| a.timestamp == b.timestamp);

                let curr = pts.last().map(|p| p.value).unwrap_or(0.0);
                let th_val = th.warning_threshold.or(th.critical_threshold).unwrap_or(100.0);
                let above = matches!(th.comparison.as_str(), "gt"|"gte");

                let resp = match self.call_profeta(&dev.device_id, metric, &pts).await {
                    Ok(r) => r,
                    Err(e) => { warn!("profeta {}/{}: {e}", dev.device_id, metric); continue }
                };

                let median = resp.forecast_values.get("0.5").cloned().unwrap_or_default();
                let times = resp.forecast_timestamps;
                let trend = resp.analysis.as_ref()
                    .and_then(|a| a.trend_analysis.as_ref())
                    .and_then(|t| t.trend_interpretation.as_deref()).unwrap_or("stable");
                let conf = resp.analysis.as_ref()
                    .and_then(|a| a.quality_metrics.as_ref())
                    .and_then(|q| q.prediction_stability)
                    .map(|s| (s * 100.0) as i32).unwrap_or(70);

                let breach = median.iter().position(|&v| if above { v >= th_val } else { v <= th_val });
                let (pred_val, pred_at, h) = match breach {
                    Some(i) => {
                        let ts = times.get(i).cloned().unwrap_or_default();
                        let hrs = chrono::DateTime::parse_from_rfc3339(&ts)
                            .map(|t| ((t.timestamp() - Utc::now().timestamp()) as f64 / 3600.0).max(0.0))
                            .unwrap_or(-1.0);
                        (median[i], ts, hrs)
                    }
                    None => (median.iter().cloned().fold(curr, f64::max), times.last().cloned().unwrap_or_default(), -1.0),
                };

                if h < 0.0 && pred_val < th_val * 0.8 { continue }

                buf.push(CachedPrediction {
                    id: format!("{}-{}-{}", dev.device_id, th.metric_name, Utc::now().timestamp_millis()),
                    device_id: dev.device_id.clone(), device_name: dev.device_name.clone(),
                    metric_name: th.metric_name.clone(), display_name: th.display_name.clone(),
                    current_value: (curr * 10.0).round() / 10.0,
                    predicted_value: (pred_val * 10.0).round() / 10.0,
                    threshold_value: th_val,
                    threshold_type: if th.critical_threshold.map(|c| pred_val >= c).unwrap_or(false) { "critical" } else { "warning" }.into(),
                    predicted_at: pred_at, confidence: conf, trend: trend.into(),
                    hours_until: (h * 10.0).round() / 10.0, created_at: Utc::now(),
                });
            }
        }

        self.save_predictions(&buf).await?;
        debug!("saved {} predictions", buf.len());
        Ok(())
    }

    async fn call_profeta(&self, nid: &str, metric: &str, pts: &[MetricPoint]) -> Result<ProfetaResponse> {
        let body = serde_json::json!({
            "metrics": {
                "series_name": format!("{nid}_{metric}"), "server_id": nid,
                "metric_type": metric, "unit": "%",
                "data_points": pts.iter().map(|p| serde_json::json!({"timestamp": p.timestamp, "value": p.value})).collect::<Vec<_>>()
            },
            "period_type": "day", "prediction_horizon": "4 hours",
            "num_samples": 200, "confidence_levels": [0.5, 0.9], "include_analysis": true
        });

        let r = self.http.post("http://profeta:8000/metrics/forecast/daily")
            .json(&body).timeout(Duration::from_secs(30)).send().await
            .map_err(|e| AppError::Internal(format!("profeta: {e}")))?;

        let st = r.status();
        if !st.is_success() {
            return Err(AppError::Internal(format!("profeta {st}: {}", r.text().await.unwrap_or_default())));
        }
        r.json().await.map_err(|e| AppError::Internal(format!("profeta parse: {e}")))
    }

    async fn save_predictions(&self, preds: &[CachedPrediction]) -> Result<()> {
        for p in preds {
            sqlx::query(
                "INSERT INTO prediction_cache(id,device_id,device_name,metric_name,display_name,current_value,\
                 predicted_value,threshold_value,threshold_type,predicted_at,confidence,trend,hours_until,created_at)\
                 VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14)"
            ).bind(&p.id).bind(&p.device_id).bind(&p.device_name).bind(&p.metric_name).bind(&p.display_name)
             .bind(p.current_value).bind(p.predicted_value).bind(p.threshold_value).bind(&p.threshold_type)
             .bind(&p.predicted_at).bind(p.confidence).bind(&p.trend).bind(p.hours_until).bind(p.created_at)
             .execute(&self.pool).await.map_err(AppError::Database)?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CachedPrediction {
    pub id: String,
    pub device_id: String,
    pub device_name: String,
    pub metric_name: String,
    pub display_name: Option<String>,
    pub current_value: f64,
    pub predicted_value: f64,
    pub threshold_value: f64,
    pub threshold_type: String,
    pub predicted_at: String,
    pub confidence: i32,
    pub trend: String,
    pub hours_until: f64,
    pub created_at: chrono::DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricPoint { pub timestamp: String, pub value: f64 }

#[derive(Debug, Deserialize)]
struct ProfetaResponse {
    forecast_values: HashMap<String, Vec<f64>>,
    forecast_timestamps: Vec<String>,
    analysis: Option<ProfetaAnalysis>,
}

#[derive(Debug, Deserialize)]
struct ProfetaAnalysis {
    trend_analysis: Option<TrendAnalysis>,
    #[serde(default)] quality_metrics: Option<QualityMetrics>,
}

#[derive(Debug, Deserialize)]
struct TrendAnalysis { trend_interpretation: Option<String> }

#[derive(Debug, Deserialize)]
struct QualityMetrics { prediction_stability: Option<f64> }
