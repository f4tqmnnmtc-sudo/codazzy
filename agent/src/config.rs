use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, info};

use crate::error::AgentError;
use crate::types::Seconds;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    pub node: NodeConfig,
    pub collection: CollectionConfig,
    pub transport: TransportConfig,
    pub metrics: MetricsConfig,
    pub logging: LoggingConfig,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    pub id: String,
    #[serde(default)]
    pub location: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default = "_env")]
    pub environment: String,
}

fn _env() -> String { "production".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CollectionConfig {
    #[serde(default = "_interval")]
    pub interval: Seconds,
    pub hardware: HardwareConfig,
    pub network: NetworkConfig,
    pub storage: StorageConfig,
    #[serde(default)]
    pub processes: Option<ProcessConfig>,
}

fn _interval() -> Seconds { 5 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HardwareConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub cpu_detailed: bool,
    #[serde(default)]
    pub memory_detailed: bool,
    #[serde(default = "yes")]
    pub temperature_sensors: bool,
    #[serde(default)]
    pub gpu_metrics: bool,
    #[serde(default)]
    pub power_metrics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetworkConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default = "yes")]
    pub interface_details: bool,
    #[serde(default)]
    pub bandwidth_monitoring: bool,
    #[serde(default)]
    pub latency_monitoring: bool,
    #[serde(default)]
    pub dns_metrics: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    #[serde(default = "yes")]
    pub enabled: bool,
    #[serde(default)]
    pub disk_io_detailed: bool,
    #[serde(default = "yes")]
    pub filesystem_monitoring: bool,
    #[serde(default)]
    pub smart_metrics: bool,
}

fn yes() -> bool { true }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessConfig {
    #[serde(default)]
    pub enabled: bool,
    #[serde(default = "proc_interval")]
    pub interval: Seconds,
}

fn proc_interval() -> Seconds { 300 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TransportConfig {
    pub nats_url: String,
    #[serde(default)]
    pub nats_urls: Vec<String>,
    #[serde(default = "topic")]
    pub topic_prefix: String,
    #[serde(default = "buffer")]
    pub buffer_size: usize,
    #[serde(default = "yes")]
    pub compression: bool,
    #[serde(default = "retries")]
    pub retry_attempts: u32,
    #[serde(default = "batch")]
    pub batch_size: usize,
    #[serde(default = "flush")]
    pub flush_interval: Seconds,
    #[serde(default = "conn_timeout")]
    pub conn_timeout: Seconds,
    #[serde(default)]
    pub tls: Option<TlsConfig>,
}

fn topic() -> String { "metrics".into() }
fn buffer() -> usize { 1000 }
fn retries() -> u32 { 3 }
fn batch() -> usize { 10 }
fn flush() -> Seconds { 30 }
fn conn_timeout() -> Seconds { 10 }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TlsConfig {
    pub enabled: bool,
    pub ca_file: Option<String>,
    pub cert_file: Option<String>,
    pub key_file: Option<String>,
    #[serde(default = "yes")]
    pub verify_hostname: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MetricsConfig {
    #[serde(default = "precision")]
    pub precision: String,
    #[serde(default = "retention")]
    pub retention_hours: u32,
    #[serde(default)]
    pub custom_labels: std::collections::HashMap<String, String>,
    #[serde(default = "exclude_ifaces")]
    pub exclude_interfaces: Vec<String>,
    #[serde(default = "exclude_fs")]
    pub exclude_filesystems: Vec<String>,
}

fn precision() -> String { "medium".into() }
fn retention() -> u32 { 24 }

fn exclude_ifaces() -> Vec<String> {
    vec!["lo".into(), "docker0".into(), "veth*".into(), "br-*".into(), "virbr*".into()]
}

fn exclude_fs() -> Vec<String> {
    vec!["tmpfs".into(), "devtmpfs".into(), "sysfs".into(), "proc".into(), "cgroup".into(), "overlay".into()]
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoggingConfig {
    #[serde(default = "log_level")]
    pub level: String,
    #[serde(default)]
    pub file: Option<String>,
    #[serde(default = "log_size")]
    pub max_file_size_mb: u32,
    #[serde(default = "log_files")]
    pub max_files: u32,
    #[serde(default)]
    pub json_format: bool,
}

fn log_level() -> String { "info".into() }
fn log_size() -> u32 { 10 }
fn log_files() -> u32 { 5 }

impl Default for Config {
    fn default() -> Self {
        let hostname = hostname::get()
            .map(|h| h.to_string_lossy().into_owned())
            .unwrap_or_else(|_| "unknown".into());
        
        Self {
            node: NodeConfig {
                id: hostname,
                location: None,
                tags: vec![],
                environment: "production".into(),
            },
            collection: CollectionConfig {
                interval: 5,
                hardware: HardwareConfig {
                    enabled: true,
                    cpu_detailed: false,
                    memory_detailed: false,
                    temperature_sensors: true,
                    gpu_metrics: false,
                    power_metrics: false,
                },
                network: NetworkConfig {
                    enabled: true,
                    interface_details: true,
                    bandwidth_monitoring: false,
                    latency_monitoring: false,
                    dns_metrics: false,
                },
                storage: StorageConfig {
                    enabled: true,
                    disk_io_detailed: false,
                    filesystem_monitoring: true,
                    smart_metrics: false,
                },
                processes: None,
            },
            transport: TransportConfig {
                nats_url: "nats://localhost:4222".into(),
                nats_urls: vec![],
                topic_prefix: "metrics".into(),
                buffer_size: 1000,
                compression: true,
                retry_attempts: 3,
                batch_size: 10,
                flush_interval: 30,
                conn_timeout: 10,
                tls: None,
            },
            metrics: MetricsConfig {
                precision: "medium".into(),
                retention_hours: 24,
                custom_labels: Default::default(),
                exclude_interfaces: exclude_ifaces(),
                exclude_filesystems: exclude_fs(),
            },
            logging: LoggingConfig {
                level: "info".into(),
                file: None,
                max_file_size_mb: 10,
                max_files: 5,
                json_format: false,
            },
        }
    }
}

impl Config {
    pub async fn load() -> Result<Self, AgentError> {
        let paths = Self::config_paths();
        
        for path in &paths {
            if !Path::new(path).exists() {
                continue;
            }
            
            debug!("intentando cargar config desde: {}", path);
            
            let content = fs::read_to_string(path).await
                .map_err(|e| AgentError::config(format!("error leyendo {}: {}", path, e)))?;
            
            let mut cfg: Config = toml::from_str(&content)
                .map_err(|e| AgentError::config(format!("error parseando TOML: {}", e)))?;
            
            cfg.env();
            
            info!("configuración cargada desde: {}", path);
            return Ok(cfg);
        }

        debug!("no config file found, using defaults");
        let mut cfg = Config::default();
        cfg.env();
        
        if let Some(path) = paths.first() {
            if let Err(e) = cfg.save(path).await {
                debug!("couldn't save default config: {}", e);
            }
        }
        
        Ok(cfg)
    }

    fn config_paths() -> Vec<String> {
        let mut paths = vec!["config.toml".into()];
        
        #[cfg(target_os = "linux")]
        {
            paths.push("/opt/codazzy/config/config.toml".into());
            if let Ok(home) = std::env::var("HOME") {
                paths.push(format!("{}/.config/codazzy/agent/config.toml", home));
            }
            paths.push("/etc/codazzy/agent/config.toml".into());
        }
        
        #[cfg(target_os = "macos")]
        {
            if let Ok(home) = std::env::var("HOME") {
                paths.push(format!("{}/.config/codazzy/agent/config.toml", home));
                paths.push(format!("{}/Library/Application Support/Codazzy/agent/config.toml", home));
            }
            paths.push("/usr/local/etc/codazzy/agent/config.toml".into());
        }
        
        #[cfg(target_os = "windows")]
        {
            if let Ok(appdata) = std::env::var("APPDATA") {
                paths.push(format!("{}\\Codazzy\\agent\\config.toml", appdata));
            }
            paths.push("C:\\ProgramData\\Codazzy\\agent\\config.toml".into());
        }
        
        paths
    }

    fn env(&mut self) {
        if let Ok(v) = std::env::var("CODAZZY_NODE_ID") { self.node.id = v; }
        if let Ok(v) = std::env::var("CODAZZY_NATS_URL") { self.transport.nats_url = v; }
        if let Ok(v) = std::env::var("CODAZZY_LOG_LEVEL") { self.logging.level = v; }
        if let Ok(v) = std::env::var("CODAZZY_ENVIRONMENT") { self.node.environment = v; }
        if let Ok(v) = std::env::var("CODAZZY_LOCATION") { self.node.location = Some(v); }
        
        if let Ok(v) = std::env::var("CODAZZY_COLLECTION_INTERVAL") {
            if let Ok(n) = v.parse() { self.collection.interval = n; }
        }
    }

    async fn save(&self, path: &str) -> Result<(), AgentError> {
        if let Some(parent) = Path::new(path).parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).await
                    .map_err(|e| AgentError::config(format!("mkdir: {}", e)))?;
            }
        }
        
        let content = toml::to_string_pretty(self)
            .map_err(|e| AgentError::config(format!("toml serialize: {}", e)))?;
        
        fs::write(path, content).await
            .map_err(|e| AgentError::config(format!("write {}: {}", path, e)))
    }

    pub fn validate(&self) -> Result<(), AgentError> {
        if self.node.id.is_empty() {
            return Err(AgentError::config("node.id no puede estar vacío"));
        }
        
        if self.node.id.len() > 128 {
            return Err(AgentError::config("node.id demasiado largo (max 128 chars)"));
        }
        
        if self.collection.interval == 0 {
            return Err(AgentError::config("interval debe ser > 0"));
        }
        
        if self.transport.nats_url.is_empty() {
            return Err(AgentError::config("nats_url es requerido"));
        }
        
        if !self.transport.nats_url.starts_with("nats://") && !self.transport.nats_url.starts_with("tls://") {
            return Err(AgentError::config(
                "nats_url debe empezar con nats:// o tls://"
            ));
        }
        
        let v_precisions = ["low", "medium", "high"];
        if !v_precisions.contains(&self.metrics.precision.as_str()) {
            return Err(AgentError::config(format!(
                "precision '{}' inválida, usar: {}",
                self.metrics.precision,
                v_precisions.join(", ")
            )));
        }
        
        Ok(())
    }
}
