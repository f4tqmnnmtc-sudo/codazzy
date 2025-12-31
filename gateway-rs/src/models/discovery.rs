use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Cancelled,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryDeviceStatus {
    Discovered,
    Configured,
    AgentInstalled,
    Ignored,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Hash)]
#[serde(rename_all = "snake_case")]
pub enum ProtocolType {
    Snmp,
    Ssh,
    Http,
    Https,
    Mqtt,
    Telnet,
    Ftp,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum DiscoveryDeviceType {
    Server,
    Router,
    Switch,
    Firewall,
    AccessPoint,
    Printer,
    IoTDevice,
    Mobile,
    Workstation,
    #[default]
    Unknown,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CollectionMethod {
    InstallAgent,
    RemoteSnmp,
    RemoteSsh,
    RemoteApi,
    RemoteMqtt,
    NotRecommended,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanConfiguration {
    pub target_ranges: Vec<String>,
    pub protocols: Vec<ProtocolType>,
    #[serde(default)]
    pub options: ScanOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanOptions {
    #[serde(default = "default_true")]
    pub detect_topology: bool,
    #[serde(default = "default_true")]
    pub identify_devices: bool,
    #[serde(default)]
    pub port_scan: bool,
    #[serde(default)]
    pub aggressive_scan: bool,
    #[serde(default = "default_timeout")]
    pub timeout_seconds: u32,
}

fn default_true() -> bool {
    true
}
fn default_timeout() -> u32 {
    5
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub percentage: f64,
    pub current_ip: String,
    pub ips_scanned: u32,
    pub total_ips: u32,
    pub elapsed_seconds: u64,
    pub estimated_remaining_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScanResults {
    #[serde(default)]
    pub devices_found: usize,
    #[serde(default)]
    pub protocols_detected: HashMap<String, usize>,
    #[serde(default)]
    pub errors: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatusResponse {
    pub scan_id: String,
    pub status: ScanStatus,
    pub progress: ScanProgress,
    pub results: ScanResults,
    pub current_phase: String,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub completed_at: Option<DateTime<Utc>>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionRecommendation {
    pub recommended_method: CollectionMethod,
    pub confidence: f64,
    pub reasons: Vec<String>,
    #[serde(default)]
    pub alternative_methods: Vec<CollectionMethod>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveryDetails {
    pub scan_id: String,
    pub discovery_method: Vec<String>,
    pub response_times: HashMap<String, f64>,
    pub confidence_score: f64,
    #[serde(default)]
    pub flags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String,
    pub ip_address: String,
    #[serde(default)]
    pub mac_address: Option<String>,
    #[serde(default)]
    pub hostname: Option<String>,
    pub status: DiscoveryDeviceStatus,
    pub device_type: DiscoveryDeviceType,
    #[serde(default)]
    pub vendor: Option<String>,
    #[serde(default)]
    pub os: Option<String>,
    pub discovered_at: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    #[serde(default)]
    pub open_ports: Vec<u16>,
    #[serde(default)]
    pub available_protocols: Vec<ProtocolType>,
    #[serde(default)]
    pub collection_recommendation: Option<CollectionRecommendation>,
    #[serde(default)]
    pub discovery_details: Option<DiscoveryDetails>,
    #[serde(default)]
    pub scan_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyNode {
    pub id: String,
    pub label: String,
    pub device_type: DiscoveryDeviceType,
    pub ip_address: String,
    #[serde(default)]
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TopologyEdge {
    pub source_id: String,
    pub target_id: String,
    #[serde(default)]
    pub connection_type: String,
    #[serde(default)]
    pub bandwidth: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct TopologyStatistics {
    pub total_nodes: usize,
    pub total_edges: usize,
    #[serde(default)]
    pub device_type_counts: HashMap<String, usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkTopology {
    pub scan_id: String,
    pub nodes: Vec<TopologyNode>,
    pub edges: Vec<TopologyEdge>,
    pub statistics: TopologyStatistics,
    pub generated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StartScanRequest {
    pub target_ranges: Vec<String>,
    #[serde(default)]
    pub protocols: Vec<ProtocolType>,
    #[serde(default)]
    pub options: ScanOptions,
}

impl From<StartScanRequest> for ScanConfiguration {
    fn from(req: StartScanRequest) -> Self {
        ScanConfiguration {
            target_ranges: req.target_ranges,
            protocols: if req.protocols.is_empty() {
                vec![
                    ProtocolType::Snmp,
                    ProtocolType::Ssh,
                    ProtocolType::Http,
                    ProtocolType::Https,
                ]
            } else {
                req.protocols
            },
            options: req.options,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestRequest {
    pub protocol: ProtocolType,
    pub connection: ConnectionDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionDetails {
    pub port: u16,
    #[serde(default)]
    pub credentials: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestResult {
    pub status: String,
    #[serde(default)]
    pub response_time_ms: Option<f64>,
    pub message: String,
    #[serde(default)]
    pub sample_data: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionTestResponse {
    pub device_id: String,
    pub protocol: ProtocolType,
    pub test_results: HashMap<String, TestResult>,
    pub overall_status: String,
    pub confidence: f64,
    pub recommendations: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfigurationRequest {
    pub configuration_type: String,
    pub metadata: DeviceMetadata,
    #[serde(default)]
    pub remote_config: Option<RemoteConnectionConfig>,
    #[serde(default)]
    pub agent_install: Option<AgentInstallConfig>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceMetadata {
    pub name: String,
    pub device_type: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteConnectionConfig {
    pub protocol: ProtocolType,
    pub connection: ConnectionDetails,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentInstallConfig {
    pub ssh_connection: SshConnectionDetails,
    pub install_options: InstallOptions,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SshConnectionDetails {
    pub port: u16,
    pub username: String,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub private_key: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallOptions {
    pub os_type: String,
    pub nats_url: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceConfigurationResponse {
    pub device_id: String,
    pub configuration_status: String,
    pub monitoring_type: String,
    pub new_id: String,
    pub message: String,
    #[serde(default)]
    pub error: Option<String>,
    #[serde(default)]
    pub next_collection: Option<DateTime<Utc>>,
}
