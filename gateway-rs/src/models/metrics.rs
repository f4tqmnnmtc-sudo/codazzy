use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemMetrics {
    pub node_id: String,
    pub timestamp: i64,
    pub hardware: HardwareMetrics,
    pub network: NetworkMetrics,
    pub storage: StorageMetrics,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub processes: Option<ProcessSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub device_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub connection_protocol: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub location: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub teleco_specific: Option<HashMap<String, HashMap<String, f64>>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct HardwareMetrics {
    #[serde(default)]
    pub cpu_usage: Vec<f64>,
    #[serde(default)]
    pub memory_usage: MemoryUsage,
    #[serde(default)]
    pub thermal_sensors: Vec<ThermalSensor>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct MemoryUsage {
    pub total: u64,
    pub used: u64,
    pub available: u64,
    pub cached: u64,
    pub buffers: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThermalSensor {
    pub name: String,
    pub temperature: f64,
    pub critical_temp: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct NetworkMetrics {
    #[serde(default)]
    pub interfaces: Vec<NetworkInterface>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkInterface {
    pub name: String,
    pub bytes_sent: u64,
    pub bytes_received: u64,
    pub packets_sent: u64,
    pub packets_received: u64,
    pub errors_in: u64,
    pub errors_out: u64,
    #[serde(default = "ret_true")]
    pub is_up: bool,
}

fn ret_true() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct StorageMetrics {
    #[serde(default)]
    pub disks: Vec<DiskMetrics>,
    #[serde(default)]
    pub filesystems: Vec<FilesystemMetrics>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiskMetrics {
    pub name: String,
    pub read_bytes: u64,
    pub write_bytes: u64,
    pub read_ops: u64,
    pub write_ops: u64,
    pub utilization: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilesystemMetrics {
    pub mount_point: String,
    pub total_space: u64,
    pub used_space: u64,
    pub available_space: u64,
    pub usage_percent: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ProcessSummary {
    pub total_processes: u32,
    pub running_processes: u32,
    pub sleeping_processes: u32,
    pub zombie_processes: u32,
    pub total_threads: u32,
    #[serde(default)]
    pub top_cpu_processes: Vec<ProcessMetrics>,
    #[serde(default)]
    pub top_memory_processes: Vec<ProcessMetrics>,
    #[serde(default)]
    pub detected_services: Vec<ServiceInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessMetrics {
    pub pid: u32,
    pub name: String,
    pub exe_path: Option<String>,
    #[serde(default)]
    pub cmd: Vec<String>,
    pub cpu_usage: f64,
    pub memory_bytes: u64,
    pub memory_percent: f64,
    #[serde(default)]
    pub status: String,
    pub user: Option<String>,
    pub start_time: Option<i64>,
    pub threads: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: Option<String>,
    #[serde(default)]
    pub status: String,
    pub process_count: u32,
    pub total_cpu: f64,
    pub total_memory: u64,
    #[serde(default)]
    pub pids: Vec<u32>,
    pub exe_path: Option<String>,
    pub command: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct OverviewMetrics {
    pub timestamp: DateTime<Utc>,
    pub time_range: String,
    pub system_health: SystemHealth,
    pub active_nodes: usize,
    pub performance: PerfMetrics,
    pub alerts: AlertCounts,
    pub metrics: MiscMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SystemHealth {
    pub score: u8,
    pub status: String,
    pub trend: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerfMetrics {
    pub cpu_avg: f64,
    pub memory_avg: f64,
    pub network_throughput: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AlertCounts {
    pub critical: u32,
    pub warning: u32,
    pub info: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiscMetrics {
    pub metrics_per_minute: u64,
    pub temperature_avg: f64,
    pub network_errors: u64,
    pub throughput_trend: String,
    pub latency_p95: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInfo {
    pub id: String,
    pub name: String,
    #[serde(rename = "type")]
    pub agent_type: String,
    pub type_icon: String,
    pub location: String,
    pub last_seen: String,
    pub last_seen_seconds: i64,
    pub status: String,
    pub status_icon: String,
    pub cpu_usage: f64,
    pub memory_usage: f64,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentsListResponse {
    pub agents: Vec<AgentInfo>,
    pub total_count: usize,
    pub online_count: usize,
    pub degraded_count: usize,
    pub offline_count: usize,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeMetrics {
    pub node_id: String,
    pub timestamp: DateTime<Utc>,
    pub hardware: HashMap<String, Vec<MetricDataPoint>>,
    pub network: HashMap<String, Vec<MetricDataPoint>>,
    pub storage: HashMap<String, Vec<MetricDataPoint>>,
    pub data_points: Vec<MetricDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricDataPoint {
    pub time: DateTime<Utc>,
    pub value: f64,
    pub component: Option<String>,
    pub metric_type: Option<String>,
    pub node_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TimeSeriesData {
    pub data_points: Vec<MetricDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CacheStats {
    pub cache_methods: HashMap<String, MethodCacheStats>,
    pub total_cached_methods: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MethodCacheStats {
    pub size: usize,
    pub max_size: usize,
    pub ttl: u64,
}
