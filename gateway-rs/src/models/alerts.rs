use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AlertSeverity {
    Info,
    Warning,
    Critical,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfig {
    pub metric_name: String,
    pub warning_threshold: Option<f64>,
    pub critical_threshold: Option<f64>,
    #[serde(default)]
    pub comparison: String,
    #[serde(default)]
    pub duration_seconds: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceThresholds {
    pub device_id: String,
    pub thresholds: Vec<ThresholdConfig>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdConfigAI {
    pub metric_name: String,
    pub display_name: Option<String>,
    pub unit: Option<String>,
    pub warning_threshold: Option<f64>,
    pub critical_threshold: Option<f64>,
    #[serde(default = "comparison")]
    pub comparison: String,
    #[serde(default = "priority")]
    pub priority: String,
    pub reasoning: Option<String>,
    pub ai_model: Option<String>,
    #[serde(default = "enabled")]
    pub enabled: bool,
}

fn comparison() -> String { "gt".into() }
fn priority() -> String { "medium".into() }
fn enabled() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceThresholdsAI {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    pub thresholds: Vec<ThresholdConfigAI>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Anomaly {
    pub node_id: String,
    pub timestamp: i64,
    pub metric_name: String,
    pub value: f64,
    pub severity: AlertSeverity,
    pub threshold_warning: Option<f64>,
    pub threshold_critical: Option<f64>,
    pub detected_at: DateTime<Utc>,
    #[serde(default)]
    pub metadata: HashMap<String, serde_json::Value>,
}
