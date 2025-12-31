use crate::error::{AppError, Result};
use crate::models::teleco_device::{ConnectionConfig, DeviceType};
use crate::services::ssh_service::{SshConfig, SshService};
use regex::Regex;
use std::collections::HashMap;

pub mod commands {
    pub const MOBILE_CELL_STATUS: &str = "show cell status";
    pub const MOBILE_KPI_STATS: &str = "show kpi statistics";
    pub const MOBILE_USER_COUNT: &str = "show active-users count";
    pub const MOBILE_PERFORMANCE: &str = "show performance counters";
    pub const FIBER_OPTICAL_POWER: &str = "show interface optical-power";
    pub const FIBER_LINE_STATUS: &str = "show line status";
    pub const FIBER_ERROR_COUNTERS: &str = "show interface error-counters";
    pub const SAT_SIGNAL_QUALITY: &str = "show satellite signal";
    pub const SAT_MODEM_STATUS: &str = "show modem status";
    pub const SAT_LINK_STATUS: &str = "show link status";
    pub const IOT_GATEWAY_STATUS: &str = "show gateway status";
    pub const IOT_DEVICE_LIST: &str = "show devices";
    pub const IOT_TRAFFIC_STATS: &str = "show traffic statistics";
    pub const LINUX_UNAME: &str = "uname -a";
    pub const LINUX_UPTIME: &str = "uptime";
    pub const LINUX_MEMORY: &str = "free -m";
    pub const LINUX_DISK: &str = "df -h";
    pub const LINUX_CPU_TOP: &str = "top -bn1 | head -5";
    pub const LINUX_PROCESSES: &str = "ps aux --sort=-%cpu | head -10";
}

pub struct SshCollector { host: String, port: u16, user: String, pwd: Option<String>, key: Option<String>, timeout: u64 }

impl SshCollector {
    pub fn new(cfg: &ConnectionConfig) -> Self {
        Self { host: cfg.host.clone(), port: cfg.port, user: cfg.credentials.get("username").cloned().unwrap_or_else(|| "root".into()), pwd: cfg.credentials.get("password").cloned(), key: cfg.credentials.get("private_key").cloned(), timeout: cfg.additional_params.get("timeout").and_then(|v| v.as_u64()).unwrap_or(30) }
    }

    fn cfg(&self) -> SshConfig { SshConfig { hostname: self.host.clone(), port: self.port, username: self.user.clone(), password: self.pwd.clone(), private_key: self.key.clone(), timeout_secs: self.timeout } }
    pub async fn test_connection(&self) -> Result<bool> { SshService::new().test_connection(&self.cfg()).await }
    pub async fn execute_command(&self, cmd: &str) -> Result<String> { Ok(SshService::new().execute_command(&self.cfg(), cmd).await?.stdout) }

    pub async fn collect_metrics(&self, cmds: &[String]) -> Result<HashMap<String, f64>> {
        let mut m = HashMap::new();
        for cmd in cmds { if let Ok(out) = self.execute_command(cmd).await { if let Ok(v) = out.trim().parse::<f64>() { m.insert(cmd.split_whitespace().last().unwrap_or("unknown").into(), v); } } }
        Ok(m)
    }

    pub async fn collect_system_metrics(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new();
        if let Ok(o) = self.execute_command(commands::LINUX_UNAME).await { m.insert("system_info".into(), serde_json::Value::String(o.trim().into())); }
        if let Ok(o) = self.execute_command(commands::LINUX_UPTIME).await { if let Some(p) = self.parse_uptime(&o) { m.extend(p); } }
        if let Ok(o) = self.execute_command(commands::LINUX_MEMORY).await { if let Some(p) = self.parse_memory(&o) { m.extend(p); } }
        if let Ok(o) = self.execute_command(commands::LINUX_DISK).await { if let Some(p) = self.parse_disk(&o) { m.extend(p); } }
        if let Ok(o) = self.execute_command(commands::LINUX_CPU_TOP).await { if let Some(p) = self.parse_cpu(&o) { m.extend(p); } }
        if let Ok(o) = self.execute_command(commands::LINUX_PROCESSES).await { m.insert("process_count".into(), serde_json::json!(o.lines().count().saturating_sub(1))); }
        Ok(m)
    }

    fn parse_uptime(&self, o: &str) -> Option<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new();
        if let Some(c) = Regex::new(r"up\s+(?:(\d+)\s+days?,?\s*)?(?:(\d+):(\d+))?").ok()?.captures(o) {
            let (d, h, mn): (u64, u64, u64) = (c.get(1).and_then(|x| x.as_str().parse().ok()).unwrap_or(0), c.get(2).and_then(|x| x.as_str().parse().ok()).unwrap_or(0), c.get(3).and_then(|x| x.as_str().parse().ok()).unwrap_or(0));
            m.insert("uptime_seconds".into(), serde_json::json!(d * 86400 + h * 3600 + mn * 60));
        }
        if let Some(c) = Regex::new(r"load average:\s*([\d.]+),?\s*([\d.]+)?,?\s*([\d.]+)?").ok()?.captures(o) {
            m.insert("load_average".into(), serde_json::json!({ "1m": c.get(1).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0), "5m": c.get(2).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0), "15m": c.get(3).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0) }));
        }
        Some(m)
    }

    fn parse_memory(&self, o: &str) -> Option<HashMap<String, serde_json::Value>> {
        let c = Regex::new(r"Mem:\s+(\d+)\s+(\d+)\s+(\d+)(?:\s+\d+\s+\d+\s+(\d+))?").ok()?.captures(o)?;
        let (t, u, f): (f64, f64, f64) = (c.get(1).and_then(|x| x.as_str().parse().ok())?, c.get(2).and_then(|x| x.as_str().parse().ok())?, c.get(3).and_then(|x| x.as_str().parse().ok())?);
        let a = c.get(4).and_then(|x| x.as_str().parse().ok()).unwrap_or(f);
        Some(HashMap::from([("memory_total_mb".into(), serde_json::json!(t)), ("memory_used_mb".into(), serde_json::json!(u)), ("memory_free_mb".into(), serde_json::json!(f)), ("memory_available_mb".into(), serde_json::json!(a)), ("memory_usage_percent".into(), serde_json::json!(if t > 0.0 { (u / t) * 100.0 } else { 0.0 }))]))
    }

    fn parse_disk(&self, o: &str) -> Option<HashMap<String, serde_json::Value>> {
        let re = Regex::new(r"(?:/dev/\S+|\S+)\s+(\d+(?:\.\d+)?[KMGT]?)\s+(\d+(?:\.\d+)?[KMGT]?)\s+(\d+(?:\.\d+)?[KMGT]?)\s+(\d+)%\s+/$").ok()?;
        for ln in o.lines() { if let Some(c) = re.captures(ln) { return Some(HashMap::from([("disk_usage_percent".into(), serde_json::json!(c.get(4).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0))), ("disk_total_mb".into(), serde_json::json!(c.get(1).and_then(|x| self.parse_size(x.as_str())).unwrap_or(0.0))), ("disk_used_mb".into(), serde_json::json!(c.get(2).and_then(|x| self.parse_size(x.as_str())).unwrap_or(0.0))), ("disk_available_mb".into(), serde_json::json!(c.get(3).and_then(|x| self.parse_size(x.as_str())).unwrap_or(0.0)))])) } }
        None
    }

    fn parse_size(&self, s: &str) -> Option<f64> {
        let s = s.trim(); let lc = s.chars().last()?;
        let (n, mul) = if lc.is_alphabetic() { (&s[..s.len() - 1], match lc.to_ascii_uppercase() { 'K' => 1.0 / 1024.0, 'M' => 1.0, 'G' => 1024.0, 'T' => 1024.0 * 1024.0, _ => 1.0 / 1024.0 }) } else { (s, 1.0 / 1024.0) };
        n.parse::<f64>().ok().map(|v| v * mul)
    }

    fn parse_cpu(&self, o: &str) -> Option<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new();
        for ln in o.lines() {
            if let Some(c) = Regex::new(r"%?Cpu\(s\):\s*([\d.]+)%?\s*us").ok()?.captures(ln) { m.insert("cpu_user_percent".into(), serde_json::json!(c.get(1).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0))); }
            if let Some(c) = Regex::new(r"([\d.]+)%?\s*id").ok()?.captures(ln) { m.insert("cpu_usage_percent".into(), serde_json::json!(100.0 - c.get(1).and_then(|x| x.as_str().parse::<f64>().ok()).unwrap_or(0.0))); }
        }
        Some(m)
    }

    pub async fn collect_device_metrics(&self, dt: &DeviceType) -> Result<HashMap<String, serde_json::Value>> {
        match dt { DeviceType::MobileInfrastructure => self.collect_mobile().await, DeviceType::FiberIsp => self.collect_fiber().await, DeviceType::Satellite => self.collect_sat().await, DeviceType::IotGateway => self.collect_iot().await, DeviceType::Standard => Ok(HashMap::new()) }
    }

    async fn collect_mobile(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new(); let mut raw: HashMap<String, String> = HashMap::new();
        for (nm, cmd) in [("cell_status", commands::MOBILE_CELL_STATUS), ("kpi_stats", commands::MOBILE_KPI_STATS), ("user_count", commands::MOBILE_USER_COUNT), ("performance", commands::MOBILE_PERFORMANCE)] { if let Ok(o) = self.execute_command(cmd).await { raw.insert(nm.into(), o); } }
        if let Some(o) = raw.get("cell_status") { if let Some(v) = self.extract_val(o, &["Cell ID", "CellId"]) { m.insert("cell_id".into(), serde_json::Value::String(v)); } if let Some(v) = self.extract_val(o, &["Technology", "RAT"]) { m.insert("technology".into(), serde_json::Value::String(v)); } }
        if let Some(o) = raw.get("kpi_stats") { for (pats, k) in [(&["RSRP", "Reference Signal"][..], "rsrp"), (&["RSRQ", "Quality"], "rsrq"), (&["SINR", "Signal.*Noise"], "sinr")] { if let Some(v) = self.extract_num(o, pats) { m.insert(k.into(), serde_json::json!(v)); } } }
        if let Some(o) = raw.get("user_count") { if let Some(v) = self.extract_num(o, &["Active", "Users", "Count"]) { m.insert("active_users".into(), serde_json::json!(v)); } }
        m.insert("raw_command_results".into(), serde_json::to_value(&raw).unwrap_or(serde_json::Value::Null)); Ok(m)
    }

    async fn collect_fiber(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new(); let mut raw: HashMap<String, String> = HashMap::new();
        for (nm, cmd) in [("optical_power", commands::FIBER_OPTICAL_POWER), ("line_status", commands::FIBER_LINE_STATUS), ("error_counters", commands::FIBER_ERROR_COUNTERS)] { if let Ok(o) = self.execute_command(cmd).await { raw.insert(nm.into(), o); } }
        if let Some(o) = raw.get("optical_power") { if let Some(v) = self.extract_num(o, &["RX", "Receive", "Input"]) { m.insert("optical_power_rx".into(), serde_json::json!(v)); } if let Some(v) = self.extract_num(o, &["TX", "Transmit", "Output"]) { m.insert("optical_power_tx".into(), serde_json::json!(v)); } }
        if let Some(o) = raw.get("line_status") { if let Some(v) = self.extract_num(o, &["SNR", "Signal.*Noise"]) { m.insert("snr".into(), serde_json::json!(v)); } }
        m.insert("raw_command_results".into(), serde_json::to_value(&raw).unwrap_or(serde_json::Value::Null)); Ok(m)
    }

    async fn collect_sat(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new(); let mut raw: HashMap<String, String> = HashMap::new();
        for (nm, cmd) in [("signal_quality", commands::SAT_SIGNAL_QUALITY), ("modem_status", commands::SAT_MODEM_STATUS), ("link_status", commands::SAT_LINK_STATUS)] { if let Ok(o) = self.execute_command(cmd).await { raw.insert(nm.into(), o); } }
        if let Some(o) = raw.get("signal_quality") { for (pats, k) in [(&["SNR", "Signal.*Noise"][..], "snr"), (&["Es/N0", "Symbol.*Noise"], "es_n0")] { if let Some(v) = self.extract_num(o, pats) { m.insert(k.into(), serde_json::json!(v)); } } }
        if let Some(o) = raw.get("modem_status") { for (pats, k) in [(&["TX.*Power", "Transmit"][..], "tx_power"), (&["RX.*Power", "Receive"], "rx_power")] { if let Some(v) = self.extract_num(o, pats) { m.insert(k.into(), serde_json::json!(v)); } } }
        m.insert("raw_command_results".into(), serde_json::to_value(&raw).unwrap_or(serde_json::Value::Null)); Ok(m)
    }

    async fn collect_iot(&self) -> Result<HashMap<String, serde_json::Value>> {
        let mut m = HashMap::new(); let mut raw: HashMap<String, String> = HashMap::new();
        for (nm, cmd) in [("gateway_status", commands::IOT_GATEWAY_STATUS), ("device_list", commands::IOT_DEVICE_LIST), ("traffic_stats", commands::IOT_TRAFFIC_STATS)] { if let Ok(o) = self.execute_command(cmd).await { raw.insert(nm.into(), o); } }
        if let Some(o) = raw.get("gateway_status") { if let Some(v) = self.extract_num(o, &["Battery", "Power"]) { m.insert("battery_level".into(), serde_json::json!(v)); } if let Some(v) = self.extract_num(o, &["Temperature", "Temp"]) { m.insert("device_temperature".into(), serde_json::json!(v)); } }
        if let Some(o) = raw.get("device_list") { m.insert("active_devices_count".into(), serde_json::json!(o.lines().filter(|l| l.contains(':') && l.split(':').count() >= 3).count())); }
        if let Some(o) = raw.get("traffic_stats") { if let Some(v) = self.extract_num(o, &["Received", "RX", "Input"]) { m.insert("packets_received".into(), serde_json::json!(v)); } if let Some(v) = self.extract_num(o, &["Transmitted", "TX", "Output"]) { m.insert("packets_transmitted".into(), serde_json::json!(v)); } }
        m.insert("raw_command_results".into(), serde_json::to_value(&raw).unwrap_or(serde_json::Value::Null)); Ok(m)
    }

    fn extract_val(&self, txt: &str, pats: &[&str]) -> Option<String> { for p in pats { if let Some(c) = Regex::new(&format!(r"{}[:\s]+(\S+)", p)).ok()?.captures(txt) { return c.get(1).map(|x| x.as_str().trim().into()) } } None }
    fn extract_num(&self, txt: &str, pats: &[&str]) -> Option<f64> { for p in pats { if let Some(c) = Regex::new(&format!(r"{}[:\s]*([-\d.]+)", p)).ok()?.captures(txt) { return c.get(1).and_then(|x| x.as_str().parse().ok()) } } None }

    pub async fn upload_file(&self, local: &str, remote: &str) -> Result<()> {
        let c = tokio::fs::read_to_string(local).await.map_err(|e| AppError::Internal(format!("No se pudo leer {local}: {e}")))?;
        SshService::new().write_file(&self.cfg(), remote, &c).await
    }

    pub async fn download_file(&self, remote: &str, local: &str) -> Result<()> {
        let c = SshService::new().read_file(&self.cfg(), remote).await?;
        tokio::fs::write(local, &c).await.map_err(|e| AppError::Internal(format!("No se pudo escribir {local}: {e}")))
    }

    pub async fn collect_all_metrics(&self, dt: &DeviceType) -> Result<HashMap<String, serde_json::Value>> {
        let mut all = HashMap::new();
        if let Ok(sys) = self.collect_system_metrics().await { all.extend(sys); }
        if let Ok(dev) = self.collect_device_metrics(dt).await { all.extend(dev); }
        all.insert("collection_timestamp".into(), serde_json::Value::String(chrono::Utc::now().to_rfc3339())); Ok(all)
    }

    pub fn get_supported_device_types() -> Vec<DeviceType> { vec![DeviceType::Standard, DeviceType::MobileInfrastructure, DeviceType::FiberIsp, DeviceType::Satellite, DeviceType::IotGateway] }
}
