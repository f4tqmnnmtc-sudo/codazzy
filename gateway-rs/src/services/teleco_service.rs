use crate::collectors::mqtt_collector::MqttCollector;
use crate::collectors::snmp_collector::SnmpCollector;
use crate::collectors::ssh_collector::SshCollector;
use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::models::teleco_device::*;
use crate::services::cache_service::CacheService;
use chrono::Utc;
use dashmap::DashMap;
use parking_lot::RwLock;
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::task::JoinHandle;

pub struct TelecoService {
    devices: DashMap<String, TelecoDevice>,
    cache: Arc<CacheService>,
    collection_tasks: DashMap<String, JoinHandle<()>>,
    running: RwLock<bool>,
    stats: TelecoStats,
}

struct TelecoStats {
    total_collections: AtomicU64,
    successful_collections: AtomicU64,
    failed_collections: AtomicU64,
}

impl TelecoService {
    pub fn new(cache: Arc<CacheService>) -> Arc<Self> {
        Arc::new(Self {
            devices: DashMap::new(),
            cache,
            collection_tasks: DashMap::new(),
            running: RwLock::new(false),
            stats: TelecoStats {
                total_collections: AtomicU64::new(0),
                successful_collections: AtomicU64::new(0),
                failed_collections: AtomicU64::new(0),
            },
        })
    }

    pub async fn initialize(self: &Arc<Self>) -> Result<()> {
        let devices: Vec<(String, TelecoDevice)> = self.cache.hgetall("teleco:devices").await?;
        for (id, device) in devices {
            self.devices.insert(id.clone(), device);
            let _ = self.start_device_collection(&id).await;
        }
        *self.running.write() = true;
        Ok(())
    }

    pub async fn add_device(self: &Arc<Self>, request: AddDeviceRequest) -> Result<TelecoDevice> {
        let device: TelecoDevice = request.into();
        self.cache.hset("teleco:devices", &device.id, &device).await?;
        self.devices.insert(device.id.clone(), device.clone());
        self.start_device_collection(&device.id).await?;
        Ok(device)
    }

    pub async fn remove_device(&self, device_id: &str) -> Result<bool> {
        self.stop_device_collection(device_id).await;
        self.cache.hdel("teleco:devices", device_id).await?;
        Ok(self.devices.remove(device_id).is_some())
    }

    pub fn all_devices(&self) -> Vec<TelecoDevice> {
        self.devices.iter().map(|d| d.clone()).collect()
    }

    async fn start_device_collection(self: &Arc<Self>, device_id: &str) -> Result<()> {
        let device = self.devices.get(device_id).map(|d| d.clone()).ok_or_else(|| {
            AppError::NotFound(format!("Dispositivo no encontrado: {}", device_id))
        })?;

        let interval = device.metrics_config.collection_interval as u64;
        let device_id_owned = device_id.to_string();
        let self_clone = self.clone();

        let handle = tokio::spawn(async move {
            let mut timer = tokio::time::interval(tokio::time::Duration::from_secs(interval));
            loop {
                timer.tick().await;
                if !*self_clone.running.read() { break; }
                let _ = self_clone.collect_device_metrics(&device_id_owned).await;
            }
        });

        self.collection_tasks.insert(device_id.to_string(), handle);
        Ok(())
    }

    async fn stop_device_collection(&self, device_id: &str) {
        if let Some((_, handle)) = self.collection_tasks.remove(device_id) {
            handle.abort();
        }
    }

    async fn collect_device_metrics(&self, device_id: &str) -> Result<CollectionResult> {
        let start = std::time::Instant::now();
        self.stats.total_collections.fetch_add(1, Ordering::Relaxed);

        let device = match self.devices.get(device_id) {
            Some(d) => d.clone(),
            None => return Ok(CollectionResult {
                device_id: device_id.to_string(), success: false, metrics_count: 0,
                duration_ms: 0, error_message: Some("Dispositivo no encontrado".to_string()),
                timestamp: Utc::now(),
            }),
        };

        let result = match device.connection_config.protocol {
            ConnectionProtocol::Snmp => self.collect_snmp_metrics(&device).await,
            ConnectionProtocol::HttpApi => self.collect_http_metrics(&device).await,
            ConnectionProtocol::Ssh => self.collect_ssh_metrics(&device).await,
            ConnectionProtocol::Mqtt => self.collect_mqtt_metrics(&device).await,
            _ => Err(AppError::Internal(format!("Protocolo no soportado: {:?}", device.connection_config.protocol))),
        };

        let duration_ms = start.elapsed().as_millis() as u64;

        match result {
            Ok(metrics_count) => {
                self.stats.successful_collections.fetch_add(1, Ordering::Relaxed);
                if let Some(mut dev) = self.devices.get_mut(device_id) {
                    dev.status = DeviceStatus::Online;
                    dev.last_seen = Some(Utc::now());
                    dev.last_error = None;
                    dev.metrics_count = Some(metrics_count as u32);
                    dev.collection_duration_ms = Some(duration_ms as u32);
                    let device_clone = dev.clone();
                    drop(dev);
                    let _ = self.cache.hset("teleco:devices", device_id, &device_clone).await;
                }
                Ok(CollectionResult {
                    device_id: device_id.to_string(), success: true, metrics_count, duration_ms,
                    error_message: None, timestamp: Utc::now(),
                })
            }
            Err(e) => {
                self.stats.failed_collections.fetch_add(1, Ordering::Relaxed);
                if let Some(mut dev) = self.devices.get_mut(device_id) {
                    dev.status = DeviceStatus::Error;
                    dev.last_error = Some(e.to_string());
                    let device_clone = dev.clone();
                    drop(dev);
                    let _ = self.cache.hset("teleco:devices", device_id, &device_clone).await;
                }
                Ok(CollectionResult {
                    device_id: device_id.to_string(), success: false, metrics_count: 0, duration_ms,
                    error_message: Some(e.to_string()), timestamp: Utc::now(),
                })
            }
        }
    }

    async fn collect_snmp_metrics(&self, device: &TelecoDevice) -> Result<usize> {
        let collector = SnmpCollector::new(&device.connection_config);
        match collector.test_connection().await {
            Ok(true) => {}
            Ok(false) => return Err(AppError::Internal(format!("Conexion SNMP fallida para {}", device.device_name))),
            Err(e) => return Err(AppError::Internal(format!("Error SNMP para {}: {}", device.device_name, e))),
        }

        let metrics = collector.collect_all_metrics(&device.device_type).await
            .map_err(|e| AppError::Internal(format!("Recoleccion SNMP fallida: {}", e)))?;

        let metrics_count = metrics.len();
        let _ = self.cache.set(&format!("teleco:metrics:{}", device.id), &metrics).await;
        let _ = self.write_metrics_to_influx(device, &metrics).await;
        Ok(metrics_count)
    }

    async fn write_metrics_to_influx(&self, device: &TelecoDevice, metrics: &HashMap<String, serde_json::Value>) -> Result<()> {
        if metrics.is_empty() { return Ok(()); }

        let timestamp_nanos = Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut lines = Vec::new();

        let device_id = device.device_id.replace(' ', "\\ ").replace(',', "\\,");
        let device_name = device.device_name.replace(' ', "\\ ").replace(',', "\\,");
        let protocol = format!("{:?}", device.connection_config.protocol).to_lowercase();
        let device_type = format!("{:?}", device.device_type).to_lowercase();
        let location = device.location.as_deref().unwrap_or("unknown").replace(' ', "\\ ").replace(',', "\\,");

        for (metric_name, value) in metrics {
            match value {
                serde_json::Value::Number(n) => {
                    if let Some(val) = n.as_f64() {
                        lines.push(format!(
                            "metrics_v2,node_id={},device_name={},metric_type=teleco,component={},protocol={},device_type={},location={},source=teleco value={} {}",
                            device_id, device_name, metric_name.replace(' ', "_").replace(',', "_"), protocol, device_type, location, val, timestamp_nanos
                        ));
                    }
                }
                serde_json::Value::String(s) => {
                    if let Ok(val) = s.parse::<f64>() {
                        lines.push(format!(
                            "metrics_v2,node_id={},device_name={},metric_type=teleco,component={},protocol={},device_type={},location={},source=teleco value={} {}",
                            device_id, device_name, metric_name.replace(' ', "_").replace(',', "_"), protocol, device_type, location, val, timestamp_nanos
                        ));
                    }
                }
                serde_json::Value::Object(obj) => {
                    for (key, inner_value) in obj {
                        let component = format!("{}_{}", metric_name, key);
                        let val = match inner_value {
                            serde_json::Value::Number(n) => n.as_f64(),
                            serde_json::Value::String(s) => s.parse().ok(),
                            _ => None,
                        };
                        if let Some(v) = val {
                            lines.push(format!(
                                "metrics_v2,node_id={},device_name={},metric_type=teleco,component={},protocol={},device_type={},location={},source=teleco value={} {}",
                                device_id, device_name, component.replace(' ', "_").replace(',', "_"), protocol, device_type, location, v, timestamp_nanos
                            ));
                        }
                    }
                }
                _ => {}
            }
        }

        if lines.is_empty() { return Ok(()); }

        let url = format!("{}/api/v2/write?org={}&bucket={}&precision=ns", CONFIG.influx_url, CONFIG.influx_org, CONFIG.influx_bucket);
        let resp = reqwest::Client::new()
            .post(&url)
            .header("Authorization", format!("Token {}", CONFIG.influx_token))
            .header("Content-Type", "text/plain; charset=utf-8")
            .body(lines.join("\n"))
            .send().await
            .map_err(|e| AppError::InfluxDb(format!("Error escribiendo metricas teleco: {}", e)))?;

        if !resp.status().is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(AppError::InfluxDb(format!("Write fallo: {}", text)));
        }
        Ok(())
    }

    async fn collect_http_metrics(&self, device: &TelecoDevice) -> Result<usize> {
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(device.connection_config.timeout_seconds as u64))
            .build()
            .map_err(|e| AppError::Internal(format!("No se pudo crear cliente HTTP: {}", e)))?;

        let mut metrics: HashMap<String, serde_json::Value> = HashMap::new();
        let mut successful_endpoints = 0;

        for endpoint in &device.metrics_config.api_endpoints {
            let url = format!("http://{}:{}{}", device.connection_config.host, device.connection_config.port, endpoint);
            if let Ok(response) = client.get(&url).send().await {
                if response.status().is_success() {
                    if let Ok(json) = response.json::<serde_json::Value>().await {
                        if let Some(obj) = json.as_object() {
                            for (key, value) in obj {
                                metrics.insert(format!("http_{}_{}", endpoint.trim_matches('/'), key), value.clone());
                            }
                        }
                        successful_endpoints += 1;
                    }
                }
            }
        }

        metrics.insert("collection_timestamp".to_string(), serde_json::Value::String(Utc::now().to_rfc3339()));
        metrics.insert("successful_endpoints".to_string(), serde_json::json!(successful_endpoints));

        let metrics_count = metrics.len();
        let _ = self.cache.set(&format!("teleco:metrics:{}", device.id), &metrics).await;
        let _ = self.write_metrics_to_influx(device, &metrics).await;
        Ok(metrics_count)
    }

    async fn collect_ssh_metrics(&self, device: &TelecoDevice) -> Result<usize> {
        let collector = SshCollector::new(&device.connection_config);
        match collector.test_connection().await {
            Ok(true) => {}
            Ok(false) => return Err(AppError::Internal(format!("Conexion SSH fallida para {}", device.device_name))),
            Err(e) => return Err(AppError::Internal(format!("Error SSH para {}: {}", device.device_name, e))),
        }

        let metrics = collector.collect_all_metrics(&device.device_type).await
            .map_err(|e| AppError::Internal(format!("Recoleccion SSH fallida: {}", e)))?;

        let metrics_count = metrics.len();
        let _ = self.cache.set(&format!("teleco:metrics:{}", device.id), &metrics).await;
        let _ = self.write_metrics_to_influx(device, &metrics).await;
        Ok(metrics_count)
    }

    async fn collect_mqtt_metrics(&self, device: &TelecoDevice) -> Result<usize> {
        let collector = MqttCollector::new(&device.connection_config);
        match collector.test_connection().await {
            Ok(true) => {}
            Ok(false) => return Err(AppError::Internal(format!("Conexion MQTT fallida para {}", device.device_name))),
            Err(e) => return Err(AppError::Internal(format!("Error MQTT para {}: {}", device.device_name, e))),
        }

        collector.start_collection().await
            .map_err(|e| AppError::Internal(format!("Recoleccion MQTT fallida: {}", e)))?;

        let timeout = std::cmp::min(device.metrics_config.collection_interval as u64, 15);
        tokio::time::sleep(tokio::time::Duration::from_secs(timeout)).await;

        let mqtt_metrics = collector.get_metrics().await;
        let mut metrics: HashMap<String, serde_json::Value> = mqtt_metrics.into_iter().map(|(k, v)| (k, serde_json::json!(v))).collect();
        metrics.insert("collection_timestamp".to_string(), serde_json::Value::String(Utc::now().to_rfc3339()));

        let metrics_count = metrics.len();
        let _ = self.cache.set(&format!("teleco:metrics:{}", device.id), &metrics).await;
        let _ = self.write_metrics_to_influx(device, &metrics).await;
        Ok(metrics_count)
    }
}
