use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceType {
    #[default]
    Standard,
    MobileInfrastructure,
    FiberIsp,
    Satellite,
    IotGateway,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ConnectionProtocol {
    Snmp,
    HttpApi,
    Ssh,
    Telnet,
    Mqtt,
    Tr069,
    Cwmp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DeviceStatus {
    #[default]
    Unknown,
    Online,
    Offline,
    Error,
    Maintenance,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    pub protocol: ConnectionProtocol,
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub credentials: HashMap<String, String>,
    #[serde(default)]
    pub additional_params: HashMap<String, serde_json::Value>,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
    #[serde(default = "default_retries")]
    pub retry_attempts: u32,
}

fn default_timeout() -> u32 { 30 }
fn default_retries() -> u32 { 3 }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MetricsConfig {
    #[serde(default)]
    pub enabled_metrics: Vec<String>,
    #[serde(default = "default_collection_interval")]
    pub collection_interval: u32,
    #[serde(default)]
    pub custom_oids: HashMap<String, String>,
    #[serde(default)]
    pub api_endpoints: Vec<String>,
    #[serde(default)]
    pub ssh_commands: Vec<String>,
}

fn default_collection_interval() -> u32 { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TelecoDevice {
    #[serde(default = "generate_uuid")]
    pub id: String,
    pub device_id: String,
    pub device_type: DeviceType,
    pub device_name: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub connection_config: ConnectionConfig,
    #[serde(default)]
    pub metrics_config: MetricsConfig,
    #[serde(default)]
    pub status: DeviceStatus,
    #[serde(default)]
    pub last_seen: Option<DateTime<Utc>>,
    #[serde(default)]
    pub last_error: Option<String>,
    #[serde(default)]
    pub metrics_count: Option<u32>,
    #[serde(default)]
    pub collection_duration_ms: Option<u32>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "Utc::now")]
    pub created_at: DateTime<Utc>,
    #[serde(default = "Utc::now")]
    pub updated_at: DateTime<Utc>,
}

fn generate_uuid() -> String { Uuid::new_v4().to_string() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionResult {
    pub device_id: String,
    pub success: bool,
    #[serde(default)]
    pub metrics_count: usize,
    #[serde(default)]
    pub duration_ms: u64,
    #[serde(default)]
    pub error_message: Option<String>,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AddDeviceRequest {
    pub device_id: String,
    pub device_type: DeviceType,
    pub device_name: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub connection_config: ConnectionConfig,
    #[serde(default)]
    pub metrics_config: MetricsConfig,
    #[serde(default)]
    pub tags: Vec<String>,
}

impl From<AddDeviceRequest> for TelecoDevice {
    fn from(req: AddDeviceRequest) -> Self {
        TelecoDevice {
            id: Uuid::new_v4().to_string(),
            device_id: req.device_id,
            device_type: req.device_type,
            device_name: req.device_name,
            location: req.location,
            description: req.description,
            connection_config: req.connection_config,
            metrics_config: req.metrics_config,
            status: DeviceStatus::Unknown,
            last_seen: None,
            last_error: None,
            metrics_count: None,
            collection_duration_ms: None,
            tags: req.tags,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }
}
