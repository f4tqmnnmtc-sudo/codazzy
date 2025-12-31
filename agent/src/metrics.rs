use crate::collectors::processes::ProcessSummary;
use crate::types::*;
use serde::Serialize;

#[derive(Debug, Serialize, Clone)]
pub struct SystemMetrics {
    pub node_id: String,
    pub timestamp: UnixTs,
    pub hardware: HardwareMetrics,
    pub network: NetworkMetrics,
    pub storage: StorageMetrics,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub processes: Option<ProcessSummary>,
}

impl SystemMetrics {
    pub fn new(node_id: &str) -> Self {
        Self {
            node_id: node_id.to_owned(),
            timestamp: chrono::Utc::now().timestamp(),
            hardware: HardwareMetrics::default(),
            network: NetworkMetrics::default(),
            storage: StorageMetrics::default(),
            processes: None,
        }
    }
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct HardwareMetrics {
    pub cpu_usage: Vec<Percent>,
    pub memory_usage: MemoryInfo,
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub thermal_sensors: Vec<ThermalReading>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct MemoryInfo {
    pub total: Bytes,
    pub used: Bytes,
    pub available: Bytes,
    pub cached: Bytes,
    pub buffers: Bytes,
}

#[derive(Debug, Serialize, Clone)]
pub struct ThermalReading {
    pub name: String,
    pub temperature: Celsius,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub critical_temp: Option<Celsius>,
}

#[derive(Debug, Serialize, Clone, Default)]
pub struct NetworkMetrics {
    pub interfaces: Vec<InterfaceStats>,
}

#[derive(Debug, Serialize, Clone)]
pub struct InterfaceStats {
    pub name: String,
    pub bytes_sent: Bytes,
    pub bytes_received: Bytes,
    pub packets_sent: Packets,
    pub packets_received: Packets,
    pub errors_in: Packets,
    pub errors_out: Packets,
    pub is_up: bool,
}

pub type NetworkInterface = InterfaceStats;

#[derive(Debug, Serialize, Clone, Default)]
pub struct StorageMetrics {
    pub disks: Vec<DiskStats>,
    pub filesystems: Vec<FilesystemStats>,
}

#[derive(Debug, Serialize, Clone)]
pub struct DiskStats {
    pub name: String,
    pub read_bytes: Bytes,
    pub write_bytes: Bytes,
    pub read_ops: Opes,
    pub write_ops: Opes,
    pub utilization: Percent,
}

#[derive(Debug, Serialize, Clone)]
pub struct FilesystemStats {
    pub mount_point: String,
    pub total_space: Bytes,
    pub used_space: Bytes,
    pub available_space: Bytes,
    pub usage_percent: Percent,
}
