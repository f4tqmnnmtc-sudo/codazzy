use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::services::CacheService;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::{collections::HashMap, net::Ipv4Addr, process::Stdio, sync::Arc, time::Duration};
use tokio::{process::Command, sync::RwLock, time::timeout};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScanStatus { #[default] Pending, Running, Completed, Failed, Cancelled }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanProgress {
    pub percentage: f64, pub current_ip: String, pub ips_scanned: u32,
    pub total_ips: u32, pub elapsed_seconds: u64, pub estimated_remaining_seconds: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanResults { pub devices_found: u32, pub protocols_detected: HashMap<String, u32>, pub errors: Vec<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScanStatusResponse {
    pub scan_id: String, pub status: ScanStatus, pub progress: ScanProgress, pub results: ScanResults,
    pub current_phase: String, pub started_at: DateTime<Utc>, pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")] pub completed_at: Option<DateTime<Utc>>,
    #[serde(skip_serializing_if = "Option::is_none")] pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DiscoveredDevice {
    pub id: String, pub ip_address: String, pub mac_address: Option<String>, pub hostname: Option<String>,
    pub status: String, pub device_type: String, pub vendor: Option<String>, pub os: Option<String>,
    pub discovered_at: DateTime<Utc>, pub last_seen: DateTime<Utc>, pub open_ports: Vec<u16>,
    pub available_protocols: Vec<String>, pub scan_id: String, pub response_time_ms: f64,
    #[serde(skip_serializing_if = "Option::is_none")] pub description: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub container_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub container_name: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub container_image: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub container_status: Option<String>,
    #[serde(default)] pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")] pub location: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub environment: Option<String>,
    #[serde(default)] pub tags: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub notes: Option<String>,
}

#[derive(Debug, Clone)]
struct ActiveScan { #[allow(dead_code)] scan_id: String, #[allow(dead_code)] target_ranges: Vec<String>, cancel_flag: Arc<RwLock<bool>> }

#[derive(Debug, Clone, Serialize, Deserialize)]
struct DiscoveredHost { ip: String, mac: Option<String>, hostname: Option<String>, is_up: bool, response_time_ms: f64, open_ports: Vec<u16>, device_type: String, vendor: Option<String> }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DockerContainer { pub id: String, pub name: String, pub image: String, pub status: String, pub ip_address: Option<String>, pub ports: Vec<String>, pub networks: Vec<String>, pub created: String }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeviceCounts { pub total: usize, pub network_scan: usize, pub docker_container: usize, pub manual: usize, pub discovered: usize, pub configured: usize, pub agent_installed: usize }

pub struct DiscoveryService { cache: Arc<CacheService>, active_scans: Arc<RwLock<HashMap<String, ActiveScan>>>, db_pool: Option<PgPool> }

impl DiscoveryService {
    pub fn new(cache: Arc<CacheService>) -> Self { Self { cache, active_scans: Arc::new(RwLock::new(HashMap::new())), db_pool: None } }
    pub fn with_db(cache: Arc<CacheService>, pool: PgPool) -> Self { Self { cache, active_scans: Arc::new(RwLock::new(HashMap::new())), db_pool: Some(pool) } }

    pub async fn scan_docker_containers(&self) -> Result<Vec<DockerContainer>> {
        let out = Command::new("docker").args(["ps", "-a", "--format", "{{json .}}"]).output().await;
        match out {
            Ok(o) if o.status.success() => {
                let stdout = String::from_utf8_lossy(&o.stdout);
                let mut cs = Vec::new();
                for ln in stdout.lines().filter(|l| !l.trim().is_empty()) {
                    if let Ok(j) = serde_json::from_str::<serde_json::Value>(ln) {
                        let cid = j["ID"].as_str().unwrap_or("").to_string();
                        let ports = j["Ports"].as_str().unwrap_or("");
                        cs.push(DockerContainer {
                            id: cid.clone(), name: j["Names"].as_str().unwrap_or("").into(),
                            image: j["Image"].as_str().unwrap_or("").into(), status: j["Status"].as_str().unwrap_or("").into(),
                            ip_address: self.docker_inspect(&cid, "{{range .NetworkSettings.Networks}}{{.IPAddress}}{{end}}").await,
                            ports: if ports.is_empty() { vec![] } else { ports.split(',').map(|s| s.trim().into()).collect() },
                            networks: self.docker_inspect(&cid, "{{range $k, $v := .NetworkSettings.Networks}}{{$k}} {{end}}").await
                                .map(|s| s.split_whitespace().map(String::from).collect()).unwrap_or_default(),
                            created: j["CreatedAt"].as_str().unwrap_or("").into(),
                        });
                    }
                }
                Ok(cs)
            }
            Ok(o) => Err(AppError::Internal(format!("docker: {}", String::from_utf8_lossy(&o.stderr)))),
            Err(e) => Err(AppError::Internal(format!("docker n/a: {e}"))),
        }
    }

    async fn docker_inspect(&self, cid: &str, fmt: &str) -> Option<String> {
        Command::new("docker").args(["inspect", "-f", fmt, cid]).output().await.ok()
            .filter(|o| o.status.success())
            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_string())
            .filter(|s| !s.is_empty())
    }

    fn container_to_device(&self, c: &DockerContainer, scan_id: &str) -> DiscoveredDevice {
        let id = format!("docker-{}", c.id.chars().take(12).collect::<String>());
        let ip = c.ip_address.clone().unwrap_or_else(|| "172.17.0.x".into());
        let ports: Vec<u16> = c.ports.iter().filter_map(|p| p.split("->").last()?.split('/').next()?.parse().ok()).collect();
        let protos: Vec<String> = ports.iter().filter_map(|p| match p { 22 => Some("ssh"), 80|3000 => Some("http"), 443 => Some("https"), 5432 => Some("postgresql"), 6379 => Some("redis"), 8086 => Some("influxdb"), 4222 => Some("nats"), _ => None }).map(Into::into).collect();
        let now = Utc::now();
        DiscoveredDevice {
            id, ip_address: ip, mac_address: None, hostname: Some(c.name.clone()),
            status: if c.status.contains("Up") { "running" } else { "stopped" }.into(),
            device_type: self.identify_container_type(&c.image, &c.name), vendor: Some("Docker".into()), os: Some(c.image.clone()),
            open_ports: ports, available_protocols: protos, response_time_ms: 0.0, source: "docker_container".into(),
            container_id: Some(c.id.clone()), container_name: Some(c.name.clone()), container_image: Some(c.image.clone()), container_status: Some(c.status.clone()),
            location: None, environment: Some("docker".into()), tags: c.networks.clone(), notes: None,
            discovered_at: now, last_seen: now, scan_id: scan_id.into(), description: None,
        }
    }

    fn identify_container_type(&self, img: &str, nm: &str) -> String {
        let haystack = format!("{} {}", img.to_lowercase(), nm.to_lowercase());

        // Posibles patrones según tecnología enfocada
        static PATTERNS: &[(&[&str], &str)] = &[
            (&["postgres", "mysql", "mongo", "mariadb"], "database"),
            (&["redis", "memcached"], "cache"),
            (&["influx", "timescale", "questdb"], "timeseries-db"),
            (&["nats", "kafka", "rabbitmq", "pulsar"], "message-queue"),
            (&["mqtt", "mosquitto"], "iot-gateway"),
            (&["nginx", "apache", "caddy"], "web-server"),
            (&["traefik", "envoy"], "proxy"),
            (&["haproxy", "fabio"], "load-balancer"),
            (&["gateway"], "api-gateway"),
            (&["api", "backend"], "api-server"),
            (&["chronos", "forecast", "prophet", "ml"], "ml-service"),
            (&["prometheus", "grafana", "monitor", "alertmanager"], "monitoring"),
            (&["snmp"], "snmp-device"),
            (&["test", "mock"], "test-server"),
            (&["dashboard", "frontend", "ui"], "frontend"),
        ];

        PATTERNS.iter()
            .find(|(kws, _)| kws.iter().any(|kw| haystack.contains(kw)))
            .map(|(_, t)| (*t).into())
            .unwrap_or_else(|| "container".into())
    }

    pub async fn start_scan(&self, ranges: Vec<String>) -> Result<String> {
        let scan_id = format!("scan-{}", &Uuid::new_v4().to_string()[..8]);
        for r in &ranges { if !self.validate_ip_range(r) { return Err(AppError::Validation(format!("bad range: {r}"))) } }

        let total: u32 = ranges.iter().map(|r| self.host_count(r)).sum();
        let st = ScanStatusResponse {
            scan_id: scan_id.clone(), status: ScanStatus::Running,
            progress: ScanProgress { percentage: 0.0, current_ip: String::new(), ips_scanned: 0, total_ips: total, elapsed_seconds: 0, estimated_remaining_seconds: 0 },
            results: ScanResults { devices_found: 0, protocols_detected: HashMap::new(), errors: vec![] },
            current_phase: "init".into(), started_at: Utc::now(), updated_at: Utc::now(), completed_at: None, error: None,
        };
        self.save_scan_status(&scan_id, &st).await?;

        let cancel = Arc::new(RwLock::new(false));
        self.active_scans.write().await.insert(scan_id.clone(), ActiveScan { scan_id: scan_id.clone(), target_ranges: ranges.clone(), cancel_flag: cancel.clone() });

        let (cache, scans, pool, sid) = (self.cache.clone(), self.active_scans.clone(), self.db_pool.clone(), scan_id.clone());
        tokio::spawn(async move { let _ = DiscoveryService { cache, active_scans: scans, db_pool: pool }.execute_scan(&sid, ranges, cancel).await; });
        Ok(scan_id)
    }

    async fn execute_scan(&self, scan_id: &str, ranges: Vec<String>, cancel: Arc<RwLock<bool>>) -> Result<()> {
        let t0 = Utc::now();
        let (mut found, mut scanned) = (0u32, 0u32);
        let total: u32 = ranges.iter().map(|r| self.host_count(r)).sum();

        self.update_scan_phase(scan_id, "docker").await?;
        if let Ok(cs) = self.scan_docker_containers().await {
            for (i, c) in cs.iter().enumerate() {
                if *cancel.read().await { break }
                self.save_device(scan_id, &self.container_to_device(c, scan_id)).await?;
                found += 1;
                self.update_scan_progress(scan_id, (i+1) as f64 / cs.len() as f64 * 30.0, &c.name, (i+1) as u32, total+10, (Utc::now()-t0).num_seconds() as u64, 30).await?;
            }
        }

        for (ri, rng) in ranges.iter().enumerate() {
            if *cancel.read().await { self.update_scan_status(scan_id, ScanStatus::Cancelled, "cancelled", found).await?; return Ok(()) }
            self.update_scan_phase(scan_id, &format!("net {}/{}", ri+1, ranges.len())).await?;

            for h in self.scan_range(rng).await? {
                scanned += 1;
                if *cancel.read().await { break }
                let elapsed = (Utc::now() - t0).num_seconds() as u64;
                let pct = 30.0 + (ri as f64 / ranges.len() as f64) * 70.0;
                self.update_scan_progress(scan_id, pct.min(99.0), &h.ip, scanned, total, elapsed, if scanned > 0 && elapsed > 0 { ((elapsed as f64 / scanned as f64) * (total - scanned) as f64) as u64 } else { 30 }).await?;

                if h.is_up {
                    let now = Utc::now();
                    let dev = DiscoveredDevice {
                        id: format!("discovered-{}", h.ip.replace('.', "-")), ip_address: h.ip.clone(), mac_address: h.mac.clone(),
                        hostname: h.hostname.clone(), status: "discovered".into(), device_type: self.identify_device_type(&h.ip),
                        vendor: h.vendor.clone(), os: None, discovered_at: now, last_seen: now, open_ports: h.open_ports.clone(),
                        available_protocols: self.detect_protocols(&h.open_ports), scan_id: scan_id.into(), response_time_ms: h.response_time_ms,
                        description: None, container_id: None, container_name: None, container_image: None, container_status: None,
                        source: "network_scan".into(), location: None, environment: None, tags: vec![], notes: None,
                    };
                    self.save_device(scan_id, &dev).await?;
                    found += 1;
                }
            }
        }

        self.update_scan_status(scan_id, ScanStatus::Completed, "done", found).await?;
        self.active_scans.write().await.remove(scan_id);
        Ok(())
    }

    fn identify_device_type(&self, ip: &str) -> String { if ip.ends_with(".1") { "router" } else if ip.ends_with(".254") { "gateway" } else { "unknown" }.into() }

    async fn scan_range(&self, cidr: &str) -> Result<Vec<DiscoveredHost>> {
        let (base, plen) = self.parse_cidr(cidr)?;
        let n = 1u32 << (32 - plen);
        let b = u32::from(base);
        let (start, end) = if n > 2 { (1, n-1) } else { (0, n) };
        let ips: Vec<String> = (start..end).map(|i| Ipv4Addr::from(b + i).to_string()).collect();

        if let Ok(r) = self.fping_scan(&ips).await { return Ok(r) }
        Ok(ips.iter().filter_map(|ip| self.ping_host_sync(ip)).collect())
    }

    async fn fping_scan(&self, ips: &[String]) -> Result<Vec<DiscoveredHost>> {
        match timeout(Duration::from_secs(CONFIG.discovery_scan_timeout_secs), Command::new("fping").args(["-a","-q","-r","1","-t","500"]).args(ips).stdout(Stdio::piped()).stderr(Stdio::piped()).output()).await {
            Ok(Ok(o)) => Ok(String::from_utf8_lossy(&o.stdout).lines().filter(|l| !l.trim().is_empty())
                .map(|ip| DiscoveredHost { ip: ip.trim().into(), mac: None, hostname: None, is_up: true, response_time_ms: 0.0, open_ports: vec![], device_type: "unknown".into(), vendor: None }).collect()),
            Ok(Err(e)) => Err(AppError::Internal(format!("fping: {e}"))),
            Err(_) => Err(AppError::Internal("fping timeout".into())),
        }
    }

    fn ping_host_sync(&self, ip: &str) -> Option<DiscoveredHost> {
        let t = std::time::Instant::now();
        let up = std::process::Command::new("ping").args(["-c", "1", "-W", "1", ip]).output().ok()?.status.success();
        up.then(|| DiscoveredHost { ip: ip.into(), mac: None, hostname: None, is_up: true, response_time_ms: t.elapsed().as_secs_f64() * 1000.0, open_ports: vec![], device_type: "unknown".into(), vendor: None })
    }

    fn detect_protocols(&self, ports: &[u16]) -> Vec<String> { ports.iter().filter_map(|p| match p { 22 => Some("ssh"), 23 => Some("telnet"), 80 => Some("http"), 443 => Some("https"), 161 => Some("snmp"), 1883 => Some("mqtt"), _ => None }).map(Into::into).collect() }

    pub fn validate_ip_range(&self, r: &str) -> bool { r.split_once('/').map(|(ip, p)| ip.parse::<Ipv4Addr>().is_ok() && p.parse::<u8>().map(|x| x <= 32).unwrap_or(false)).unwrap_or(false) }

    fn parse_cidr(&self, c: &str) -> Result<(Ipv4Addr, u8)> {
        let (ip, p) = c.split_once('/').ok_or_else(|| AppError::Validation("bad cidr".into()))?;
        let addr: Ipv4Addr = ip.parse().map_err(|_| AppError::Validation("bad ip".into()))?;
        let plen: u8 = p.parse().map_err(|_| AppError::Validation("bad prefix".into()))?;
        (plen <= 32).then_some((addr, plen)).ok_or_else(|| AppError::Validation("prefix>32".into()))
    }

    pub fn host_count(&self, r: &str) -> u32 { self.parse_cidr(r).map(|(_, p)| 1u32 << (32 - p)).unwrap_or(0) }

    pub async fn scan_status(&self, id: &str) -> Result<Option<ScanStatusResponse>> { self.cache.get(&format!("discovery:scan:{id}")).await }
    pub async fn latest_scan_status(&self) -> Result<Option<ScanStatusResponse>> {
        let ids: Vec<String> = self.cache.lrange("discovery:scans", 0, 0).await.unwrap_or_default();
        match ids.first() { Some(id) => self.scan_status(id).await, None => Ok(None) }
    }

    pub async fn stop_scan(&self, id: &str) -> Result<()> {
        let scans = self.active_scans.read().await;
        let s = scans.get(id).ok_or_else(|| AppError::NotFound(format!("scan {id}")))?;
        *s.cancel_flag.write().await = true;
        Ok(())
    }

    async fn save_scan_status(&self, id: &str, st: &ScanStatusResponse) -> Result<()> {
        self.cache.set_ex(&format!("discovery:scan:{id}"), st, 86400).await?;
        self.cache.lpush("discovery:scans", id).await?;
        self.cache.ltrim("discovery:scans", 0, 9).await
    }

    async fn update_scan_phase(&self, id: &str, ph: &str) -> Result<()> {
        if let Some(mut s) = self.scan_status(id).await? { s.current_phase = ph.into(); s.updated_at = Utc::now(); self.save_scan_status(id, &s).await?; }
        Ok(())
    }

    async fn update_scan_progress(&self, id: &str, pct: f64, ip: &str, scanned: u32, total: u32, elapsed: u64, rem: u64) -> Result<()> {
        if let Some(mut s) = self.scan_status(id).await? {
            s.progress = ScanProgress { percentage: pct, current_ip: ip.into(), ips_scanned: scanned, total_ips: total, elapsed_seconds: elapsed, estimated_remaining_seconds: rem };
            s.updated_at = Utc::now(); self.save_scan_status(id, &s).await?;
        }
        Ok(())
    }

    async fn update_scan_status(&self, id: &str, st: ScanStatus, ph: &str, found: u32) -> Result<()> {
        if let Some(mut s) = self.scan_status(id).await? {
            s.status = st.clone(); s.current_phase = ph.into(); s.results.devices_found = found; s.updated_at = Utc::now();
            if matches!(st, ScanStatus::Completed | ScanStatus::Failed) { s.completed_at = Some(Utc::now()); }
            self.save_scan_status(id, &s).await?;
        }
        Ok(())
    }

    async fn save_device(&self, scan_id: &str, d: &DiscoveredDevice) -> Result<()> {
        self.cache.set_ex(&format!("discovery:device:{}", d.id), d, 86400).await?;
        self.cache.sadd(&format!("discovery:scan:{scan_id}:devices"), &d.id).await?;
        self.cache.hset("discovery:devices", &d.id, &serde_json::to_value(d)?).await?;
        if let Some(p) = &self.db_pool { self.upsert_device_db(p, d).await?; }
        Ok(())
    }

    async fn upsert_device_db(&self, pool: &PgPool, d: &DiscoveredDevice) -> Result<()> {
        let ports: Vec<i32> = d.open_ports.iter().map(|&p| p as i32).collect();
        sqlx::query(
            "INSERT INTO discovered_devices(id,ip_address,mac_address,hostname,status,device_type,vendor,os,open_ports,available_protocols,response_time_ms,source,container_id,container_name,container_image,container_status,location,environment,tags,notes,custom_fields,first_discovered_at,last_seen_at,created_at,updated_at)\
             VALUES($1,$2,$3,$4,$5,$6,$7,$8,$9,$10,$11,$12,$13,$14,$15,$16,$17,$18,$19,$20,$21,$22,$23,NOW(),NOW())\
             ON CONFLICT(id)DO UPDATE SET ip_address=EXCLUDED.ip_address,mac_address=COALESCE(EXCLUDED.mac_address,discovered_devices.mac_address),hostname=COALESCE(EXCLUDED.hostname,discovered_devices.hostname),status=EXCLUDED.status,device_type=EXCLUDED.device_type,vendor=COALESCE(EXCLUDED.vendor,discovered_devices.vendor),os=COALESCE(EXCLUDED.os,discovered_devices.os),open_ports=EXCLUDED.open_ports,available_protocols=EXCLUDED.available_protocols,response_time_ms=EXCLUDED.response_time_ms,source=EXCLUDED.source,container_id=EXCLUDED.container_id,container_name=EXCLUDED.container_name,container_image=EXCLUDED.container_image,container_status=EXCLUDED.container_status,location=COALESCE(EXCLUDED.location,discovered_devices.location),environment=COALESCE(EXCLUDED.environment,discovered_devices.environment),tags=EXCLUDED.tags,notes=COALESCE(EXCLUDED.notes,discovered_devices.notes),custom_fields=discovered_devices.custom_fields||EXCLUDED.custom_fields,last_seen_at=NOW(),updated_at=NOW()"
        ).bind(&d.id).bind(&d.ip_address).bind(&d.mac_address).bind(&d.hostname).bind(&d.status).bind(&d.device_type).bind(&d.vendor).bind(&d.os).bind(&ports).bind(&d.available_protocols).bind(d.response_time_ms).bind(&d.source).bind(&d.container_id).bind(&d.container_name).bind(&d.container_image).bind(&d.container_status).bind(&d.location).bind(&d.environment).bind(&d.tags).bind(&d.notes).bind(serde_json::json!({})).bind(d.discovered_at).bind(d.last_seen)
        .execute(pool).await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn discovered_devices(&self, scan_id: Option<&str>) -> Result<Vec<DiscoveredDevice>> {
        let sid = match scan_id {
            Some(id) => id.to_string(),
            None => self.cache.lrange("discovery:scans", 0, 0).await.unwrap_or_default().first().cloned().unwrap_or_default(),
        };
        if sid.is_empty() { return Ok(vec![]) }
        let ids: Vec<String> = self.cache.smembers(&format!("discovery:scan:{sid}:devices")).await.unwrap_or_default();
        let mut devs = Vec::with_capacity(ids.len());
        for id in ids { if let Ok(Some(d)) = self.cache.get::<DiscoveredDevice>(&format!("discovery:device:{id}")).await { devs.push(d); } }
        Ok(devs)
    }

    pub async fn all_devices(&self) -> Result<Vec<DiscoveredDevice>> {
        let Some(p) = &self.db_pool else { return Ok(vec![]) };
        Ok(sqlx::query("SELECT * FROM discovered_devices ORDER BY last_seen_at DESC").fetch_all(p).await.map_err(AppError::Database)?.into_iter().filter_map(|r| self.row_to_device(r).ok()).collect())
    }

    pub async fn devices_by_source(&self, src: &str) -> Result<Vec<DiscoveredDevice>> {
        let Some(p) = &self.db_pool else { return Ok(vec![]) };
        Ok(sqlx::query("SELECT * FROM discovered_devices WHERE source=$1 ORDER BY last_seen_at DESC").bind(src).fetch_all(p).await.map_err(AppError::Database)?.into_iter().filter_map(|r| self.row_to_device(r).ok()).collect())
    }

    pub async fn docker_devices(&self) -> Result<Vec<DiscoveredDevice>> { self.devices_by_source("docker_container").await }
    pub async fn network_devices(&self) -> Result<Vec<DiscoveredDevice>> { self.devices_by_source("network_scan").await }

    pub async fn device(&self, id: &str) -> Result<Option<DiscoveredDevice>> {
        let Some(p) = &self.db_pool else { return Ok(None) };
        sqlx::query("SELECT * FROM discovered_devices WHERE id=$1").bind(id).fetch_optional(p).await.map_err(AppError::Database)?.map(|r| self.row_to_device(r)).transpose()
    }

    pub async fn delete_device(&self, id: &str) -> Result<bool> {
        let _ = self.cache.hdel("discovery:devices", id).await;
        let _ = self.cache.delete(&format!("discovery:device:{id}")).await;
        let Some(p) = &self.db_pool else { return Ok(false) };
        Ok(sqlx::query("DELETE FROM discovered_devices WHERE id=$1").bind(id).execute(p).await.map_err(AppError::Database)?.rows_affected() > 0)
    }

    pub async fn delete_all_devices(&self) -> Result<i64> {
        let Some(p) = &self.db_pool else { return Ok(0) };
        Ok(sqlx::query("DELETE FROM discovered_devices").execute(p).await.map_err(AppError::Database)?.rows_affected() as i64)
    }

    pub async fn update_device_info(&self, id: &str, hostname: Option<String>, vendor: Option<String>, dtype: Option<String>, os: Option<String>, desc: Option<String>) -> Result<()> {
        let Some(p) = &self.db_pool else { return Ok(()) };
        let mut upd = Vec::new(); let mut vals: Vec<String> = vec![id.into()]; let mut i = 2;
        for (f, v) in [("hostname", hostname), ("vendor", vendor), ("device_type", dtype), ("os", os), ("notes", desc)] {
            if let Some(x) = v { upd.push(format!("{f}=${i}")); vals.push(x); i += 1; }
        }
        if upd.is_empty() { return Ok(()) }
        let q = format!("UPDATE discovered_devices SET {},updated_at=NOW() WHERE id=$1", upd.join(","));
        let mut qry = sqlx::query(&q); for v in vals { qry = qry.bind(v); }
        qry.execute(p).await.map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn device_counts(&self) -> Result<DeviceCounts> {
        let Some(p) = &self.db_pool else { return Ok(DeviceCounts { total: 0, network_scan: 0, docker_container: 0, manual: 0, discovered: 0, configured: 0, agent_installed: 0 }) };
        let r = sqlx::query("SELECT COUNT(*)as total,COUNT(*)FILTER(WHERE source='network_scan')as network_scan,COUNT(*)FILTER(WHERE source='docker_container')as docker_container,COUNT(*)FILTER(WHERE source='manual')as manual,COUNT(*)FILTER(WHERE status='discovered')as discovered,COUNT(*)FILTER(WHERE status='configured')as configured,COUNT(*)FILTER(WHERE status='agent_installed')as agent_installed FROM discovered_devices")
            .fetch_one(p).await.map_err(AppError::Database)?;
        Ok(DeviceCounts { total: r.get::<i64,_>("total") as usize, network_scan: r.get::<i64,_>("network_scan") as usize, docker_container: r.get::<i64,_>("docker_container") as usize, manual: r.get::<i64,_>("manual") as usize, discovered: r.get::<i64,_>("discovered") as usize, configured: r.get::<i64,_>("configured") as usize, agent_installed: r.get::<i64,_>("agent_installed") as usize })
    }

    fn row_to_device(&self, r: PgRow) -> Result<DiscoveredDevice> {
        let ports: Vec<i32> = r.get("open_ports");
        Ok(DiscoveredDevice {
            id: r.get("id"), ip_address: r.get("ip_address"), mac_address: r.get("mac_address"), hostname: r.get("hostname"),
            status: r.get("status"), device_type: r.get("device_type"), vendor: r.get("vendor"), os: r.get("os"),
            discovered_at: Self::ts(&r, "first_discovered_at"), last_seen: Self::ts(&r, "last_seen_at"),
            open_ports: ports.into_iter().map(|p| p as u16).collect(), available_protocols: r.get("available_protocols"),
            scan_id: String::new(), response_time_ms: r.get("response_time_ms"), description: r.get("notes"),
            container_id: r.get("container_id"), container_name: r.get("container_name"), container_image: r.get("container_image"), container_status: r.get("container_status"),
            source: r.get("source"), location: r.get("location"), environment: r.get("environment"), tags: r.get("tags"), notes: r.get("notes"),
        })
    }

    fn ts(r: &PgRow, c: &str) -> DateTime<Utc> { r.try_get::<DateTime<Utc>,_>(c).or_else(|_| r.try_get::<chrono::NaiveDateTime,_>(c).map(|d| DateTime::<Utc>::from_naive_utc_and_offset(d, Utc))).unwrap_or_else(|_| Utc::now()) }
}
