use crate::config::CONFIG;
use crate::services::flux::{FluxQuery, range_to_mins};
use crate::{AppError, Result};
use influxdb2::Client;
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::{collections::HashMap, sync::Arc, time::{Duration, Instant}};

type Cache = HashMap<String, (Value, Instant)>;

pub struct InfluxService {
    #[allow(dead_code)] client: Client,
    cache: Arc<RwLock<Cache>>,
    ttl: Duration,
}

impl InfluxService {
    pub fn new() -> Result<Self> {
        Ok(Self { client: Client::new(&CONFIG.influx_url, &CONFIG.influx_org, &CONFIG.influx_token), cache: Arc::new(RwLock::new(HashMap::with_capacity(64))), ttl: Duration::from_secs(CONFIG.cache_ttl) })
    }

    pub async fn test_connection(&self) -> Result<()> { self.query(&FluxQuery::new().range("1m").limit(1).build()).await.map(|_| ()) }
    pub async fn execute_query(&self, q: &str) -> Result<Value> { self.query(q).await }

    async fn query(&self, flux: &str) -> Result<Value> {
        let url = format!("{}/api/v2/query?org={}", CONFIG.influx_url, CONFIG.influx_org);
        let r = reqwest::Client::new().post(&url)
            .header("Authorization", format!("Token {}", CONFIG.influx_token))
            .header("Content-Type", "application/vnd.flux")
            .body(flux.to_string()).send().await.map_err(|e| AppError::InfluxDb(format!("req: {e}")))?;
        if !r.status().is_success() { return Err(AppError::InfluxDb(format!("status {}", r.status()))) }
        Ok(self.parse_csv(&r.text().await.map_err(|e| AppError::InfluxDb(format!("read: {e}")))?))
    }

    fn parse_csv(&self, txt: &str) -> Value {
        let mut ln = txt.lines();
        let Some(hdr) = ln.next() else { return json!([]) };
        let h: Vec<_> = hdr.split(',').collect();
        json!(ln.filter(|l| !l.trim().is_empty()).map(|l| {
            let v: Vec<_> = l.split(',').collect();
            Value::Object(h.iter().enumerate().filter_map(|(i, k)| v.get(i).map(|x| ((*k).into(), json!(*x)))).collect())
        }).collect::<Vec<_>>())
    }

    fn cached(&self, k: &str) -> Option<Value> { self.cache.read().get(k).filter(|(_, t)| t.elapsed() < self.ttl).map(|(v, _)| v.clone()) }
    fn store(&self, k: &str, v: Value) { self.cache.write().insert(k.into(), (v, Instant::now())); }
    fn xf64(&self, r: &Value) -> Option<f64> { r.as_array()?.first()?.get("_value")?.as_str()?.parse().ok() }
    fn sum(&self, r: &Value) -> i64 { r.as_array().map(|a| a.iter().filter_map(|x| x["_value"].as_str()?.parse::<i64>().ok()).sum()).unwrap_or(0) }
    fn vf64(&self, r: &Value) -> f64 { r["_value"].as_f64().or_else(|| r["_value"].as_str()?.parse().ok()).unwrap_or(0.0) }

    pub async fn metrics_overview(&self, range: &str) -> Result<Value> {
        let ck = format!("ov_{range}");
        if let Some(c) = self.cached(&ck) { return Ok(c) }

        let n = self.active_nodes(range).await.unwrap_or_default().len();

        // Queries paralelas para CPU y memoria
        let cpu_q = FluxQuery::new().range(range).measurements(&["metrics_v2", "system_metrics"])
            .raw_filter(r#"r.component=="cpu" or r._field=="cpu_usage""#)
            .fields(&["value", "cpu_usage"]).mean().build();
        let mem_q = FluxQuery::new().range(range).measurements(&["metrics_v2", "system_metrics"])
            .raw_filter(r#"r.component=="memory_percent" or r._field=="memory_percent""#)
            .fields(&["value", "memory_percent"]).mean().build();
        let cnt_q = FluxQuery::new().range(range).measurements(&["metrics_v2", "system_metrics"]).count().build();

        let cpu = self.query(&cpu_q).await.ok().and_then(|r| self.xf64(&r)).unwrap_or(0.0);
        let mem = self.query(&mem_q).await.ok().and_then(|r| self.xf64(&r)).unwrap_or(0.0);
        let total = self.query(&cnt_q).await.ok().map(|r| self.sum(&r)).unwrap_or(0);
        let mins = range_to_mins(range);

        // Calculo de health score basado en umbrales
        let (score, st) = if n == 0 { (0, "critical") }
            else if cpu > 90.0 || mem > 95.0 { (30, "critical") }
            else if cpu > 80.0 || mem > 85.0 { (60, "warning") }
            else { (85, "healthy") };

        let ov = json!({
            "timestamp": chrono::Utc::now().to_rfc3339(),
            "time_range": range,
            "system_health": { "score": score, "status": st, "trend": "stable" },
            "active_nodes": n,
            "performance": { "cpu_avg": cpu, "memory_avg": mem, "network_throughput": 0 },
            "alerts": { "critical": 0, "warning": 0, "info": 0 },
            "metrics": { "metrics_per_minute": if mins > 0 { total / mins } else { 0 } }
        });
        self.store(&ck, ov.clone());
        Ok(ov)
    }

    pub async fn agents(&self) -> Result<Value> {
        let b = &CONFIG.influx_bucket;
        let cpu_res = self.query(&format!(r#"import "influxdata/influxdb/schema"
cpu = from(bucket: "{b}") |> range(start: -5m) |> filter(fn: (r) => r._measurement == "metrics_v2") |> filter(fn: (r) => r.metric_type == "hardware" and r.component == "cpu") |> group(columns: ["node_id"]) |> last() |> keep(columns: ["node_id", "_value", "_time"])
cpu"#)).await.unwrap_or(json!([]));
        let mem_res = self.query(&FluxQuery::new().range("5m").measurement("metrics_v2").metric_type("hardware").component("memory_percent").group(&["node_id"]).last().keep(&["node_id","_value"]).build()).await.unwrap_or(json!([]));
        let net_res = self.query(&FluxQuery::new().range("5m").measurement("metrics_v2").metric_type("network").exclude_virtual_ifaces().group(&["node_id","component"]).last().keep(&["node_id","component","_value"]).build()).await.unwrap_or(json!([]));
        let rate_res = self.query(&FluxQuery::new().range("2m").measurement("metrics_v2").metric_type("network").exclude_virtual_ifaces().raw_filter(r#"r.component =~ /_bytes_sent$/ or r.component =~ /_bytes_received$/"#).group(&["node_id","component"]).derivative("1s",true).last().keep(&["node_id","component","_value"]).build()).await.unwrap_or(json!([]));

        let mem_m = self.map_vals(&mem_res);
        let (tx, rx) = self.map_net(&net_res);
        let (txr, rxr) = self.map_net_f64(&rate_res);

        let mut agents = Vec::new();
        let mut seen = std::collections::HashSet::new();
        if let Some(rows) = cpu_res.as_array() {
            for r in rows {
                let Some(nid) = r["node_id"].as_str() else { continue };
                if nid.is_empty() || !seen.insert(nid.to_string()) { continue }
                let (atype, icon) = if nid.contains("router") { ("Router","🌐") } else if nid.contains("switch") { ("Switch","🔀") } else { ("Server","🖥️") };
                let ts = r["_time"].as_str().unwrap_or("");
                agents.push(json!({"id": nid, "name": nid, "type": atype, "type_icon": icon, "location": "Datacenter-A", "last_seen": if ts.is_empty() { "Desconocido" } else { "Hace menos de 5 min" }, "status": "online", "status_icon": "🟢", "cpu_usage": (self.vf64(r)*10.0).round()/10.0, "memory_usage": (mem_m.get(nid).copied().unwrap_or(0.0)*10.0).round()/10.0, "network_tx_bytes": tx.get(nid).copied().unwrap_or(0), "network_rx_bytes": rx.get(nid).copied().unwrap_or(0), "network_tx_rate": (txr.get(nid).copied().unwrap_or(0.0)*10.0).round()/10.0, "network_rx_rate": (rxr.get(nid).copied().unwrap_or(0.0)*10.0).round()/10.0}));
            }
        }
        Ok(json!({"agents": agents, "total": agents.len()}))
    }

    fn map_vals(&self, r: &Value) -> HashMap<String, f64> { r.as_array().map(|a| a.iter().filter_map(|x| Some((x["node_id"].as_str()?.into(), self.vf64(x)))).collect()).unwrap_or_default() }

    fn map_net(&self, r: &Value) -> (HashMap<String, u64>, HashMap<String, u64>) {
        let (mut tx, mut rx) = (HashMap::new(), HashMap::new());
        if let Some(rows) = r.as_array() { for x in rows { let (Some(n), Some(c)) = (x["node_id"].as_str(), x["component"].as_str()) else { continue }; let v = self.vf64(x) as u64; if c.ends_with("_bytes_sent") { *tx.entry(n.into()).or_default() += v; } else if c.ends_with("_bytes_received") { *rx.entry(n.into()).or_default() += v; } } }
        (tx, rx)
    }

    fn map_net_f64(&self, r: &Value) -> (HashMap<String, f64>, HashMap<String, f64>) {
        let (mut tx, mut rx) = (HashMap::new(), HashMap::new());
        if let Some(rows) = r.as_array() { for x in rows { let (Some(n), Some(c)) = (x["node_id"].as_str(), x["component"].as_str()) else { continue }; let v = self.vf64(x).max(0.0); if c.ends_with("_bytes_sent") { *tx.entry(n.into()).or_default() += v; } else if c.ends_with("_bytes_received") { *rx.entry(n.into()).or_default() += v; } } }
        (tx, rx)
    }

    pub async fn active_nodes(&self, range: &str) -> Result<Vec<String>> {
        let ck = format!("nodes_{range}");
        if let Some(c) = self.cached(&ck) { return Ok(serde_json::from_value(c).unwrap_or_default()) }
        let res = self.query(&FluxQuery::new().range(range).measurements(&["metrics_v2","system_metrics"]).fields(&["value","cpu_usage"]).group(&["node_id","agent_id"]).last().keep(&["node_id","agent_id"]).build()).await?;
        let mut nodes = Vec::new(); let mut seen = std::collections::HashSet::new();
        if let Some(rows) = res.as_array() { for r in rows { if let Some(id) = r["node_id"].as_str().or_else(|| r["agent_id"].as_str()) { if !id.is_empty() && seen.insert(id.to_string()) { nodes.push(id.into()); } } } }
        self.store(&ck, json!(nodes)); Ok(nodes)
    }

    pub async fn overview_metrics(&self, range: &str) -> Result<Value> { self.metrics_overview(range).await }

    pub async fn node_metrics(&self, nid: &str, range: &str) -> Result<Value> {
        let ck = format!("node_{nid}_{range}");
        if let Some(c) = self.cached(&ck) { return Ok(c) }
        let res = self.query(&FluxQuery::new().range(range).measurements(&["metrics_v2","system_metrics"]).node_id(nid).last().build()).await?;
        let m = json!({"node_id": nid, "time_range": range, "metrics": res}); self.store(&ck, m.clone()); Ok(m)
    }

    pub async fn timeseries_data(&self, nid: &str, range: &str) -> Result<Value> {
        self.query(&FluxQuery::new().range(range).measurements(&["metrics_v2","system_metrics"]).node_id(nid).fields(&["value","cpu_usage","memory_percent"]).aggregate_window("1m","mean").build()).await
    }

    pub async fn network_interfaces(&self, nid: &str) -> Result<Value> {
        self.query(&FluxQuery::new().range("5m").measurements(&["metrics_v2","network_metrics"]).raw_filter(r#"r.metric_type=="network" or r._measurement=="network_metrics""#).node_id(nid).last().build()).await
    }

    pub async fn measurements(&self) -> Result<Vec<String>> {
        let res = self.query(&format!(r#"import "influxdata/influxdb/schema" schema.measurements(bucket: "{}")"#, CONFIG.influx_bucket)).await?;
        Ok(res.as_array().map(|a| a.iter().filter_map(|r| r["_value"].as_str().map(Into::into)).collect()).unwrap_or_default())
    }

    pub fn clear_cache(&self) { self.cache.write().clear(); }
    pub fn cache_stats(&self) -> Value { json!({"entries": self.cache.read().len(), "ttl_seconds": self.ttl.as_secs()}) }

    pub async fn timeseries(&self, _: &str, _: &str, range: &str, nid: Option<&str>) -> Result<Value> { self.timeseries_by_component("metrics_v2", "value", range, nid, None).await }

    pub async fn timeseries_by_component(&self, _: &str, _: &str, range: &str, nid: Option<&str>, comp: Option<&str>) -> Result<Value> {
        let mut qb = FluxQuery::new().range(range).measurement("metrics_v2").field("value").node_id_opt(nid);
        if let Some(c) = comp { qb = qb.component(c); }
        let res = self.query(&qb.sort(&["_time"], false).build()).await?;
        Ok(json!(res.as_array().map(|a| a.iter().filter_map(|r| { let t = r["_time"].as_str()?; let v = r["_value"].as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| r["_value"].as_f64())?; Some(json!({"time": t, "value": v, "node_id": r["node_id"].as_str().or(r["agent_id"].as_str()), "component": r["component"].as_str()})) }).collect::<Vec<_>>()).unwrap_or_default()))
    }

    pub async fn save_predictions(&self, nid: &str, mtype: &str, model: &str, preds: &[PredictionPoint]) -> Result<()> {
        if preds.is_empty() { return Ok(()) }
        let pid = uuid::Uuid::new_v4();
        let ts = chrono::Utc::now().timestamp_nanos_opt().unwrap_or(0);
        let mut lines: Vec<String> = preds.iter().map(|p| { let ns = p.timestamp * 1_000_000_000; format!("predictions,node_id={nid},metric_type={mtype},model_type={model},prediction_id={pid} value={},lower_bound={},upper_bound={},confidence={} {ns}", p.value, p.lower_bound.unwrap_or(p.value), p.upper_bound.unwrap_or(p.value), p.confidence.unwrap_or(0.95)) }).collect();
        lines.push(format!("prediction_metadata,node_id={nid},metric_type={mtype},model_type={model},prediction_id={pid} points_count={},horizon_hours={} {ts}", preds.len(), preds.len()));

        let url = format!("{}/api/v2/write?org={}&bucket={}&precision=ns", CONFIG.influx_url, CONFIG.influx_org, CONFIG.influx_bucket);
        let r = reqwest::Client::new().post(&url).header("Authorization", format!("Token {}", CONFIG.influx_token)).header("Content-Type", "text/plain; charset=utf-8").body(lines.join("\n")).send().await.map_err(|e| AppError::InfluxDb(format!("save: {e}")))?;
        if !r.status().is_success() { return Err(AppError::InfluxDb(format!("write: {}", r.text().await.unwrap_or_default()))) }
        Ok(())
    }

    pub async fn predictions(&self, nid: Option<&str>, mtype: Option<&str>, range: &str) -> Result<Value> {
        let mut f = vec![r#"r._measurement=="predictions""#.to_string()];
        if let Some(n) = nid { f.push(format!(r#"r.node_id=="{n}""#)); }
        if let Some(m) = mtype { f.push(format!(r#"r.metric_type=="{m}""#)); }
        let res = self.query(&format!(r#"import "date" from(bucket: "{b}") |> range(start: -{range}, stop: date.add(d: 30d, to: now())) |> filter(fn: (r) => {f}) |> pivot(rowKey:["_time"], columnKey: ["_field"], valueColumn: "_value")"#, b = CONFIG.influx_bucket, f = f.join(" and "))).await?;
        let preds: Vec<Value> = res.as_array().map(|a| a.iter().filter_map(|r| Some(json!({"timestamp": r["_time"].as_str(), "value": r["value"].as_str()?.parse::<f64>().ok()?, "lower_bound": r["lower_bound"].as_str().and_then(|s| s.parse::<f64>().ok()), "upper_bound": r["upper_bound"].as_str().and_then(|s| s.parse::<f64>().ok()), "confidence": r["confidence"].as_str().and_then(|s| s.parse::<f64>().ok()), "node_id": r["node_id"].as_str(), "metric_type": r["metric_type"].as_str(), "model_type": r["model_type"].as_str(), "prediction_id": r["prediction_id"].as_str()}))).collect()).unwrap_or_default();
        Ok(json!({"predictions": preds, "count": preds.len()}))
    }

    pub async fn prediction_list(&self, range: &str) -> Result<Value> {
        let res = self.query(&FluxQuery::new().range(range).measurement("prediction_metadata").pivot().sort(&["_time"], true).build()).await?;
        let preds: Vec<Value> = res.as_array().map(|a| a.iter().filter_map(|r| Some(json!({"created_at": r["_time"].as_str(), "node_id": r["node_id"].as_str(), "metric_type": r["metric_type"].as_str(), "model_type": r["model_type"].as_str(), "prediction_id": r["prediction_id"].as_str(), "points_count": r["points_count"].as_str().and_then(|s| s.parse::<i64>().ok()), "horizon_hours": r["horizon_hours"].as_str().and_then(|s| s.parse::<i64>().ok())}))).collect()).unwrap_or_default();
        Ok(json!({"predictions": preds, "count": preds.len()}))
    }

    pub async fn metric_history(&self, nid: &str, comp: &str, range: &str) -> Result<Vec<crate::services::alert_service::MetricPoint>> {
        let res = self.query(&FluxQuery::new().range(range).measurement("metrics_v2").field("value").node_id(nid).component(comp).sort(&["_time"], false).build()).await?;
        Ok(res.as_array().map(|a| a.iter().filter_map(|r| { let ts = r["_time"].as_str()?; let v = r["_value"].as_str().and_then(|s| s.parse::<f64>().ok()).or_else(|| r["_value"].as_f64())?; Some(crate::services::alert_service::MetricPoint { timestamp: ts.into(), value: v }) }).collect()).unwrap_or_default())
    }
}

#[derive(Debug, Clone, serde::Deserialize, serde::Serialize)]
pub struct PredictionPoint { pub timestamp: i64, pub value: f64, pub lower_bound: Option<f64>, pub upper_bound: Option<f64>, pub confidence: Option<f64> }
