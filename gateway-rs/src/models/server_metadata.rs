use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadata {
    pub node_id: String,
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub custom_fields: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadataResponse {
    pub success: bool,
    pub message: String,
    #[serde(default)]
    pub metadata: Option<ServerMetadata>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerMetadataListResponse {
    pub success: bool,
    pub message: String,
    pub servers: Vec<ServerMetadata>,
    pub total: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateServerMetadataRequest {
    #[serde(default)]
    pub display_name: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub contact_email: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub custom_fields: Option<serde_json::Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentConnection {
    pub id: i32,
    pub node_id: String,
    pub ssh_hostname: String,
    pub ssh_port: i32,
    #[serde(default)]
    pub ssh_username: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub agent_path: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub environment: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub os_type: Option<String>,
    #[serde(default)]
    pub installation_method: String,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub last_connected_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentConnectionRequest {
    pub node_id: String,
    pub ssh_hostname: String,
    #[serde(default = "ssh_port")]
    pub ssh_port: i32,
    #[serde(default)]
    pub ssh_username: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub agent_path: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default = "environment")]
    pub environment: String,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub os_type: Option<String>,
    #[serde(default = "installation_method")]
    pub installation_method: String,
    #[serde(default)]
    pub job_id: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

fn ssh_port() -> i32 { 22 }
fn environment() -> String { "production".into() }
fn installation_method() -> String {
    "remote_ssh".to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UpdateAgentConnectionRequest {
    #[serde(default)]
    pub ssh_hostname: Option<String>,
    #[serde(default)]
    pub ssh_port: Option<i32>,
    #[serde(default)]
    pub ssh_username: Option<String>,
    #[serde(default)]
    pub config_path: Option<String>,
    #[serde(default)]
    pub agent_path: Option<String>,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub environment: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub os_type: Option<String>,
    #[serde(default)]
    pub installation_method: Option<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedService {
    pub id: i32,
    pub node_id: String,
    pub service_name: String,
    #[serde(default)]
    pub display_name: Option<String>,
    pub status: String,
    pub process_count: i32,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
    #[serde(default)]
    pub exe_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DetectedProcess {
    pub id: i32,
    pub node_id: String,
    pub pid: i32,
    pub process_name: String,
    #[serde(default)]
    pub exe_path: Option<String>,
    pub cpu_usage: f64,
    pub memory_bytes: i64,
    pub status: String,
    pub first_seen: DateTime<Utc>,
    pub last_seen: DateTime<Utc>,
}
