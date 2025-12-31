use crate::error::{AppError, Result};
use crate::models::teleco_device::{ConnectionConfig, DeviceType};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::process::Stdio;
use tokio::process::Command;

pub mod oids {
    pub const SYS_DESCR: &str = "1.3.6.1.2.1.1.1.0";
    pub const SYS_UPTIME: &str = "1.3.6.1.2.1.1.3.0";
    pub const SYS_CONTACT: &str = "1.3.6.1.2.1.1.4.0";
    pub const SYS_NAME: &str = "1.3.6.1.2.1.1.5.0";
    pub const SYS_LOCATION: &str = "1.3.6.1.2.1.1.6.0";

    pub const IF_DESCR: &str = "1.3.6.1.2.1.2.2.1.2";
    pub const IF_OPER_STATUS: &str = "1.3.6.1.2.1.2.2.1.8";
    pub const IF_IN_UCAST_PKTS: &str = "1.3.6.1.2.1.2.2.1.11";
    pub const IF_IN_ERRORS: &str = "1.3.6.1.2.1.2.2.1.14";
    pub const IF_OUT_UCAST_PKTS: &str = "1.3.6.1.2.1.2.2.1.17";
    pub const IF_OUT_ERRORS: &str = "1.3.6.1.2.1.2.2.1.20";

    pub const IF_HC_IN_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.6";
    pub const IF_HC_OUT_OCTETS: &str = "1.3.6.1.2.1.31.1.1.1.10";
    pub const IF_HIGH_SPEED: &str = "1.3.6.1.2.1.31.1.1.1.15";

    pub const NS_EXTEND_OUTPUT: &str = "1.3.6.1.4.1.8072.1.3.2.4.1.2";

    pub const MOCK_SIGNAL_STRENGTH: &str = "1.3.6.1.4.1.99999.1.1.0";
    pub const MOCK_SIGNAL_QUALITY: &str = "1.3.6.1.4.1.99999.1.2.0";
    pub const MOCK_SNR: &str = "1.3.6.1.4.1.99999.1.3.0";
    pub const MOCK_THROUGHPUT_UPLOAD: &str = "1.3.6.1.4.1.99999.2.1.0";
    pub const MOCK_THROUGHPUT_DOWNLOAD: &str = "1.3.6.1.4.1.99999.2.2.0";
    pub const MOCK_LATENCY: &str = "1.3.6.1.4.1.99999.3.1.0";
    pub const MOCK_JITTER: &str = "1.3.6.1.4.1.99999.3.2.0";
    pub const MOCK_PACKET_LOSS: &str = "1.3.6.1.4.1.99999.3.3.0";
    pub const MOCK_ACTIVE_CONNECTIONS: &str = "1.3.6.1.4.1.99999.4.1.0";
    pub const MOCK_TEMPERATURE: &str = "1.3.6.1.4.1.99999.5.1.0";
    pub const MOCK_POWER_CONSUMPTION: &str = "1.3.6.1.4.1.99999.6.1.0";
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InterfaceMetrics {
    pub index: u32,
    pub description: String,
    pub oper_status: u8,
    pub speed: Option<u64>,
    pub bytes_in: u64,
    pub bytes_out: u64,
    pub packets_in: u64,
    pub packets_out: u64,
    pub errors_in: u64,
    pub errors_out: u64,
}

#[derive(Debug, Clone)]
pub struct SnmpCollector {
    host: String,
    port: u16,
    community: String,
    version: String,
    timeout_secs: u32,
    retries: u32,
}

impl SnmpCollector {
    pub fn new(config: &ConnectionConfig) -> Self {
        Self {
            host: config.host.clone(),
            port: config.port,
            community: config
                .credentials
                .get("community")
                .cloned()
                .unwrap_or_else(|| "public".to_string()),
            version: config
                .credentials
                .get("version")
                .cloned()
                .map(|v| v.trim_start_matches('v').to_string())
                .unwrap_or_else(|| "2c".to_string()),
            timeout_secs: config
                .additional_params
                .get("timeout")
                .and_then(|v| v.as_u64())
                .unwrap_or(5) as u32,
            retries: config
                .additional_params
                .get("retries")
                .and_then(|v| v.as_u64())
                .unwrap_or(2) as u32,
        }
    }

    pub fn with_params(host: String, port: u16, community: String, version: String) -> Self {
        Self {
            host,
            port,
            community,
            version,
            timeout_secs: 5,
            retries: 2,
        }
    }

    fn build_args(&self, oid: &str, walk: bool) -> Vec<String> {
        vec![
            format!("-v{}", self.version),
            "-c".to_string(),
            self.community.clone(),
            format!("-t{}", self.timeout_secs),
            format!("-r{}", self.retries),
            if walk {
                "-Oqn".to_string()
            } else {
                "-Oqv".to_string()
            },
            format!("{}:{}", self.host, self.port),
            oid.to_string(),
        ]
    }

    async fn snmpget(&self, oid: &str) -> Result<String> {
        let output = Command::new("snmpget")
            .args(&self.build_args(oid, false))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Error ejecutando snmpget: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout).trim().to_string())
        } else {
            Err(AppError::Internal(format!(
                "snmpget fallo para OID {}: {}",
                oid,
                String::from_utf8_lossy(&output.stderr)
            )))
        }
    }

    async fn snmpwalk(&self, base_oid: &str) -> Result<Vec<(String, String)>> {
        let output = Command::new("snmpwalk")
            .args(&self.build_args(base_oid, true))
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .output()
            .await
            .map_err(|e| AppError::Internal(format!("Error ejecutando snmpwalk: {}", e)))?;

        if output.status.success() {
            Ok(String::from_utf8_lossy(&output.stdout)
                .lines()
                .filter_map(|line| {
                    let parts: Vec<&str> = line.splitn(2, ' ').collect();
                    (parts.len() == 2).then(|| (parts[0].to_string(), parts[1].to_string()))
                })
                .collect())
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            if stderr.contains("No Such Object") || stderr.contains("No Such Instance") {
                Ok(vec![])
            } else {
                Err(AppError::Internal(format!(
                    "snmpwalk fallo para OID {}: {}",
                    base_oid, stderr
                )))
            }
        }
    }

    pub async fn test_connection(&self) -> Result<bool> {
        match self.snmpget(oids::SYS_DESCR).await {
            Ok(_) => Ok(true),
            Err(_) => Ok(false),
        }
    }

    pub async fn collect_system_metrics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metrics = HashMap::new();

        for (oid, key) in [
            (oids::SYS_DESCR, "system_sysDescr"),
            (oids::SYS_NAME, "system_sysName"),
            (oids::SYS_LOCATION, "system_sysLocation"),
            (oids::SYS_CONTACT, "system_sysContact"),
        ] {
            if let Ok(value) = self.snmpget(oid).await {
                metrics.insert(key.to_string(), serde_json::Value::String(value));
            }
        }

        if let Ok(value) = self.snmpget(oids::SYS_UPTIME).await {
            let ticks_str = value.trim_matches(|c| c == '(' || c == ')');
            if let Ok(ticks) = ticks_str.parse::<u64>() {
                metrics.insert(
                    "system_sysUpTime".to_string(),
                    serde_json::json!(ticks / 100),
                );
            }
        }

        Ok(metrics)
    }

    pub async fn collect_network_metrics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metrics = HashMap::new();
        let interface_indices = self.discover_interface_indices().await?;

        metrics.insert(
            "interface_count".to_string(),
            serde_json::json!(interface_indices.len()),
        );

        for (seq, &real_index) in interface_indices.iter().take(10).enumerate() {
            if let Ok(iface) = self.collect_interface_metrics(real_index).await {
                metrics.insert(
                    format!("interface_{}", seq + 1),
                    serde_json::to_value(&iface).unwrap_or(serde_json::Value::Null),
                );
            }
        }

        Ok(metrics)
    }

    async fn discover_interface_indices(&self) -> Result<Vec<u32>> {
        let results = self.snmpwalk(oids::IF_DESCR).await?;

        let mut indices: Vec<u32> = results
            .iter()
            .filter_map(|(oid, _)| oid.split('.').last().and_then(|s| s.parse().ok()))
            .collect();

        indices.sort();

        if indices.is_empty() {
            indices = vec![1, 2, 3, 4, 5];
        }

        Ok(indices)
    }

    async fn get_interface_u64(&self, oid: &str, index: u32) -> u64 {
        self.snmpget(&format!("{}.{}", oid, index))
            .await
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(0)
    }

    async fn collect_interface_metrics(&self, index: u32) -> Result<InterfaceMetrics> {
        Ok(InterfaceMetrics {
            index,
            description: self
                .snmpget(&format!("{}.{}", oids::IF_DESCR, index))
                .await
                .unwrap_or_else(|_| format!("interface_{}", index)),
            oper_status: self
                .snmpget(&format!("{}.{}", oids::IF_OPER_STATUS, index))
                .await
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(2),
            speed: self
                .snmpget(&format!("{}.{}", oids::IF_HIGH_SPEED, index))
                .await
                .ok()
                .and_then(|v| v.parse::<u64>().ok())
                .map(|mbps| mbps * 1_000_000),
            bytes_in: self.get_interface_u64(oids::IF_HC_IN_OCTETS, index).await,
            bytes_out: self.get_interface_u64(oids::IF_HC_OUT_OCTETS, index).await,
            packets_in: self.get_interface_u64(oids::IF_IN_UCAST_PKTS, index).await,
            packets_out: self.get_interface_u64(oids::IF_OUT_UCAST_PKTS, index).await,
            errors_in: self.get_interface_u64(oids::IF_IN_ERRORS, index).await,
            errors_out: self.get_interface_u64(oids::IF_OUT_ERRORS, index).await,
        })
    }

    pub async fn collect_device_metrics(
        &self,
        device_type: &DeviceType,
    ) -> Result<HashMap<String, f64>> {
        let mut metrics = HashMap::new();

        let device_oids: &[(&str, &str, f64)] = match device_type {
            DeviceType::MobileInfrastructure => &[
                ("1.3.6.1.4.1.9.9.999.1.4.1.0", "active_calls", 0.0),
                (oids::MOCK_SIGNAL_STRENGTH, "signal_strength", -75.0),
                ("1.3.6.1.4.1.9.9.999.1.6.2.0", "handover_success_rate", 99.5),
            ],
            DeviceType::FiberIsp => &[
                ("1.3.6.1.4.1.1.1.1.1.0", "optical_power_rx", -18.5),
                ("1.3.6.1.4.1.1.1.1.2.0", "optical_power_tx", 2.5),
            ],
            DeviceType::Satellite => &[
                ("1.3.6.1.4.1.2.1.5.1.0", "link_margin", 8.5),
                ("1.3.6.1.4.1.2.1.5.2.0", "c_n0", 45.0),
                ("1.3.6.1.4.1.2.1.6.1.0", "elevation_angle", 35.0),
            ],
            DeviceType::IotGateway => &[
                ("1.3.6.1.4.1.3.1.5.1.0", "connected_devices", 50.0),
                ("1.3.6.1.4.1.3.1.4.1.0", "messages_per_second", 100.0),
                (oids::MOCK_TEMPERATURE, "gateway_temperature", 45.0),
            ],
            DeviceType::Standard => return Ok(metrics),
        };

        for (oid, name, default) in device_oids {
            metrics.insert(
                name.to_string(),
                self.get_numeric_oid(oid).await.unwrap_or(*default),
            );
        }

        if matches!(device_type, DeviceType::FiberIsp) {
            metrics.insert("ber".to_string(), 1e-12);
        }

        Ok(metrics)
    }

    pub async fn collect_extend_metrics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut metrics = HashMap::new();

        for (oid, value) in self.snmpwalk(oids::NS_EXTEND_OUTPUT).await? {
            if let Some(metric_name) = self.extract_extend_metric_name(&oid) {
                let clean_value = value.trim_matches('"').trim();
                let parsed = clean_value
                    .parse::<f64>()
                    .map(|n| serde_json::json!(n))
                    .or_else(|_| clean_value.parse::<i64>().map(|n| serde_json::json!(n)))
                    .unwrap_or_else(|_| serde_json::Value::String(clean_value.to_string()));
                metrics.insert(format!("extend_{}", metric_name), parsed);
            }
        }

        Ok(metrics)
    }

    fn extract_extend_metric_name(&self, oid: &str) -> Option<String> {
        let base = ".1.3.6.1.4.1.8072.1.3.2.4.1.2.";
        if !oid.starts_with(base) {
            return None;
        }

        let parts: Vec<&str> = oid[base.len()..].split('.').collect();
        let length: usize = parts.first()?.parse().ok()?;

        if parts.len() < length + 2 {
            return None;
        }

        let name: String = parts[1..=length]
            .iter()
            .filter_map(|s| s.parse::<u8>().ok())
            .map(|c| c as char)
            .collect();

        (!name.is_empty()).then(|| name.replace('-', "_"))
    }

    pub async fn collect_mock_teleco_metrics(&self) -> Result<HashMap<String, f64>> {
        let mock_oids = [
            (oids::MOCK_SIGNAL_STRENGTH, "signal_strength"),
            (oids::MOCK_SIGNAL_QUALITY, "signal_quality"),
            (oids::MOCK_SNR, "snr"),
            (oids::MOCK_THROUGHPUT_UPLOAD, "throughput_upload"),
            (oids::MOCK_THROUGHPUT_DOWNLOAD, "throughput_download"),
            (oids::MOCK_LATENCY, "latency"),
            (oids::MOCK_JITTER, "jitter"),
            (oids::MOCK_PACKET_LOSS, "packet_loss"),
            (oids::MOCK_ACTIVE_CONNECTIONS, "active_connections"),
            (oids::MOCK_TEMPERATURE, "temperature"),
            (oids::MOCK_POWER_CONSUMPTION, "power_consumption"),
        ];

        let mut metrics = HashMap::new();
        for (oid, name) in mock_oids {
            if let Some(value) = self.get_numeric_oid(oid).await {
                metrics.insert(name.to_string(), value);
            }
        }

        Ok(metrics)
    }

    async fn get_numeric_oid(&self, oid: &str) -> Option<f64> {
        self.snmpget(oid).await.ok().and_then(|v| {
            v.parse()
                .ok()
                .or_else(|| v.parse::<i64>().ok().map(|i| i as f64))
        })
    }

    pub async fn collect_all_metrics(
        &self,
        device_type: &DeviceType,
    ) -> Result<HashMap<String, serde_json::Value>> {
        let mut all_metrics = HashMap::new();

        if let Ok(system) = self.collect_system_metrics().await {
            all_metrics.extend(system);
        }
        if let Ok(network) = self.collect_network_metrics().await {
            all_metrics.extend(network);
        }
        if let Ok(device) = self.collect_device_metrics(device_type).await {
            for (k, v) in device {
                all_metrics.insert(k, serde_json::json!(v));
            }
        }
        if let Ok(extend) = self.collect_extend_metrics().await {
            all_metrics.extend(extend);
        }
        if self.host.contains("snmp-teleco") || self.host == "localhost" {
            if let Ok(mock) = self.collect_mock_teleco_metrics().await {
                for (k, v) in mock {
                    all_metrics.insert(k, serde_json::json!(v));
                }
            }
        }

        Ok(all_metrics)
    }

    pub fn get_supported_device_types() -> Vec<DeviceType> {
        vec![
            DeviceType::Standard,
            DeviceType::MobileInfrastructure,
            DeviceType::FiberIsp,
            DeviceType::Satellite,
            DeviceType::IotGateway,
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_extend_metric_name() {
        let collector = SnmpCollector::with_params(
            "localhost".to_string(),
            161,
            "public".to_string(),
            "2c".to_string(),
        );
        assert_eq!(
            collector.extract_extend_metric_name(".1.3.6.1.4.1.8072.1.3.2.4.1.2.3.99.112.117.1"),
            Some("cpu".to_string())
        );
    }

    #[test]
    fn test_build_args() {
        let collector = SnmpCollector::with_params(
            "192.168.1.1".to_string(),
            161,
            "public".to_string(),
            "2c".to_string(),
        );
        let args = collector.build_args("1.3.6.1.2.1.1.1.0", false);
        assert!(args.contains(&"-v2c".to_string()));
        assert!(args.contains(&"public".to_string()));
        assert!(args.contains(&"192.168.1.1:161".to_string()));
    }
}
