use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::models::metrics::*;
use crate::processing::anomaly_detector::AnomalyDetector;
use influxdb2::models::DataPoint;
use influxdb2::Client;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::mpsc;
use tracing::{error, info};

#[derive(Debug, Clone, Default)]
pub struct WriterStats {
    pub written: u64, pub queued: u64, pub batches: u64,
    pub errors: u64, pub anomalies: u64, pub pending: u64,
}

type Tx = mpsc::Sender<Vec<DataPoint>>;
type Rx = mpsc::Receiver<Vec<DataPoint>>;

pub struct InfluxWriter {
    #[allow(dead_code)]
    client: Arc<Client>,
    detector: AnomalyDetector,
    cnt: Arc<Cnt>,
    tx: Tx,
    running: Arc<AtomicBool>,
}

struct Cnt {
    written: AtomicU64, queued: AtomicU64, batches: AtomicU64,
    errors: AtomicU64, anomalies: AtomicU64, pending: AtomicU64,
}

impl InfluxWriter {
    pub fn new() -> Self {
        let client = Arc::new(Client::new(&CONFIG.influx_url, &CONFIG.influx_org, &CONFIG.influx_token));
        let (tx, rx) = mpsc::channel::<Vec<DataPoint>>(10_000);
        let cnt = Arc::new(Cnt {
            written: AtomicU64::new(0), queued: AtomicU64::new(0), batches: AtomicU64::new(0),
            errors: AtomicU64::new(0), anomalies: AtomicU64::new(0), pending: AtomicU64::new(0),
        });
        let running = Arc::new(AtomicBool::new(true));
        Self::bg_writer(client.clone(), rx, cnt.clone(), running.clone());
        info!("influx_writer: batch={}, flush={}ms", CONFIG.batch_size, CONFIG.influx_flush_interval);
        Self { client, detector: AnomalyDetector::new(), cnt, tx, running }
    }

    fn bg_writer(client: Arc<Client>, mut rx: Rx, cnt: Arc<Cnt>, running: Arc<AtomicBool>) {
        let (bucket, batch_sz, flush_ms) = (CONFIG.influx_bucket.clone(), CONFIG.batch_size, CONFIG.influx_flush_interval);
        tokio::spawn(async move {
            let mut buf: Vec<DataPoint> = Vec::with_capacity(batch_sz * 2);
            let mut tick = tokio::time::interval(Duration::from_millis(flush_ms));
            loop {
                tokio::select! {
                    Some(pts) = rx.recv() => {
                        buf.extend(pts);
                        cnt.pending.store(buf.len() as u64, Ordering::Relaxed);
                        if buf.len() >= batch_sz { Self::flush(&client, &bucket, &mut buf, &cnt).await; }
                    }
                    _ = tick.tick() => { if !buf.is_empty() { Self::flush(&client, &bucket, &mut buf, &cnt).await; } }
                    else => { if !buf.is_empty() { Self::flush(&client, &bucket, &mut buf, &cnt).await; } break; }
                }
                if !running.load(Ordering::Relaxed) {
                    if !buf.is_empty() { Self::flush(&client, &bucket, &mut buf, &cnt).await; }
                    break;
                }
            }
        });
    }

    async fn flush(client: &Client, bucket: &str, buf: &mut Vec<DataPoint>, cnt: &Cnt) {
        let pts = std::mem::take(buf);
        let n = pts.len();
        cnt.pending.store(0, Ordering::Relaxed);
        match client.write(bucket, futures::stream::iter(pts)).await {
            Ok(_) => { cnt.written.fetch_add(n as u64, Ordering::Relaxed); cnt.batches.fetch_add(1, Ordering::Relaxed); }
            Err(e) => { cnt.errors.fetch_add(1, Ordering::Relaxed); error!("influx write ({n} pts): {e}"); }
        }
    }

    pub async fn write_metrics(&self, m: &SystemMetrics) -> Result<()> {
        let mut pts = self.to_points(m);
        let anomalies = self.detector.analyze_metrics(m);
        if !anomalies.is_empty() {
            self.cnt.anomalies.fetch_add(anomalies.len() as u64, Ordering::Relaxed);
            pts.extend(self.anomaly_points(&anomalies));
        }
        let n = pts.len() as u64;
        match self.tx.try_send(pts) {
            Ok(_) => { self.cnt.queued.fetch_add(n, Ordering::Relaxed); Ok(()) }
            Err(mpsc::error::TrySendError::Full(pts)) => {
                // Backpressure: si el channel está lleno, esperamos antes de descartar
                match tokio::time::timeout(Duration::from_millis(CONFIG.backpressure_timeout_ms), self.tx.send(pts)).await {
                    Ok(Ok(_)) => { self.cnt.queued.fetch_add(n, Ordering::Relaxed); Ok(()) }
                    Ok(Err(_)) => Err(AppError::InfluxDb("channel closed".into())),
                    Err(_) => { self.cnt.errors.fetch_add(1, Ordering::Relaxed); Ok(()) }
                }
            }
            Err(mpsc::error::TrySendError::Closed(_)) => Err(AppError::InfluxDb("writer closed".into()))
        }
    }

    fn to_points(&self, m: &SystemMetrics) -> Vec<DataPoint> {
        let mut pts = Vec::with_capacity(32);
        let (ts, nid) = (m.timestamp * 1_000_000_000, &m.node_id);
        self.hw_points(&mut pts, &m.hardware, nid, ts);
        self.net_points(&mut pts, &m.network, nid, ts);
        self.stor_points(&mut pts, &m.storage, nid, ts);
        if let Some(ref t) = m.teleco_specific { self.teleco_points(&mut pts, t, nid, ts); }
        pts
    }

    fn hw_points(&self, pts: &mut Vec<DataPoint>, hw: &HardwareMetrics, nid: &str, ts: i64) {
        for (i, &usage) in hw.cpu_usage.iter().enumerate() {
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "hardware").tag("component", format!("cpu_{i}")).field("value", usage).timestamp(ts).build() { pts.push(p); }
        }
        if !hw.cpu_usage.is_empty() {
            let avg: f64 = hw.cpu_usage.iter().sum::<f64>() / hw.cpu_usage.len() as f64;
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "hardware").tag("component", "cpu").field("value", avg).timestamp(ts).build() { pts.push(p); }
        }
        let mem = &hw.memory_usage;
        for (comp, val) in [("memory_total", mem.total as f64), ("memory_used", mem.used as f64), ("memory_available", mem.available as f64), ("memory_cached", mem.cached as f64), ("memory_buffers", mem.buffers as f64)] {
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "hardware").tag("component", comp).field("value", val).timestamp(ts).build() { pts.push(p); }
        }
        if mem.total > 0 {
            let pct = (mem.used as f64 / mem.total as f64) * 100.0;
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "hardware").tag("component", "memory_percent").field("value", pct).timestamp(ts).build() { pts.push(p); }
        }
        for s in &hw.thermal_sensors {
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "hardware").tag("component", format!("temp_{}", s.name)).field("value", s.temperature).timestamp(ts).build() { pts.push(p); }
        }
    }

    fn net_points(&self, pts: &mut Vec<DataPoint>, net: &NetworkMetrics, nid: &str, ts: i64) {
        for iface in &net.interfaces {
            if iface.name.is_empty() || iface.name.starts_with("lo") || iface.name.starts_with("docker") || iface.name.starts_with("veth") || iface.name.starts_with("br-") { continue; }
            let nm = &iface.name;
            for (metric, val) in [("bytes_sent", iface.bytes_sent as f64), ("bytes_received", iface.bytes_received as f64), ("packets_sent", iface.packets_sent as f64), ("packets_received", iface.packets_received as f64), ("errors_in", iface.errors_in as f64), ("errors_out", iface.errors_out as f64)] {
                if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "network").tag("component", format!("{nm}_{metric}")).tag("interface", nm.as_str()).field("value", val).timestamp(ts).build() { pts.push(p); }
            }
            if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "network").tag("component", format!("{nm}_status")).tag("interface", nm.as_str()).field("value", if iface.is_up { 1.0 } else { 0.0 }).timestamp(ts).build() { pts.push(p); }
        }
    }

    fn stor_points(&self, pts: &mut Vec<DataPoint>, stor: &StorageMetrics, nid: &str, ts: i64) {
        for d in &stor.disks {
            let nm = &d.name;
            for (metric, val) in [("read_bytes", d.read_bytes as f64), ("write_bytes", d.write_bytes as f64), ("read_ops", d.read_ops as f64), ("write_ops", d.write_ops as f64), ("utilization", d.utilization)] {
                if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "storage").tag("component", format!("{nm}_{metric}")).tag("disk", nm.as_str()).field("value", val).timestamp(ts).build() { pts.push(p); }
            }
        }
        for fs in &stor.filesystems {
            if fs.mount_point.starts_with("/sys") || fs.mount_point.starts_with("/proc") || fs.mount_point.starts_with("/dev") || fs.mount_point.starts_with("/run") { continue; }
            let mount = if fs.mount_point == "/" {
                "_root".to_string()
            } else {
                fs.mount_point.replace('/', "_")
            };
            for (metric, val) in [("total_space", fs.total_space as f64), ("used_space", fs.used_space as f64), ("available_space", fs.available_space as f64), ("usage_percent", fs.usage_percent)] {
                if let Ok(p) = DataPoint::builder("metrics_v2").tag("node_id", nid).tag("metric_type", "storage").tag("component", format!("fs{mount}_{metric}")).tag("mount_point", fs.mount_point.as_str()).field("value", val).timestamp(ts).build() { pts.push(p); }
            }
        }
    }

    fn teleco_points(&self, pts: &mut Vec<DataPoint>, teleco: &HashMap<String, HashMap<String, f64>>, nid: &str, ts: i64) {
        for (cat, metrics) in teleco {
            for (name, &val) in metrics {
                if let Ok(p) = DataPoint::builder("teleco_metrics").tag("node_id", nid).tag("category", cat.as_str()).tag("metric", name.as_str()).field("value", val).timestamp(ts).build() { pts.push(p); }
            }
        }
    }

    fn anomaly_points(&self, anomalies: &[crate::models::alerts::Anomaly]) -> Vec<DataPoint> {
        anomalies.iter().filter_map(|a| {
            DataPoint::builder("anomalies")
                .tag("node_id", a.node_id.as_str())
                .tag("metric_name", a.metric_name.as_str())
                .tag("severity", format!("{:?}", a.severity).to_lowercase())
                .field("value", a.value)
                .field("threshold_warning", a.threshold_warning.unwrap_or(0.0))
                .field("threshold_critical", a.threshold_critical.unwrap_or(0.0))
                .timestamp(a.timestamp * 1_000_000_000)
                .build().ok()
        }).collect()
    }

    pub fn stats(&self) -> WriterStats {
        WriterStats {
            written: self.cnt.written.load(Ordering::Relaxed),
            queued: self.cnt.queued.load(Ordering::Relaxed),
            batches: self.cnt.batches.load(Ordering::Relaxed),
            errors: self.cnt.errors.load(Ordering::Relaxed),
            anomalies: self.cnt.anomalies.load(Ordering::Relaxed),
            pending: self.cnt.pending.load(Ordering::Relaxed),
        }
    }

    pub fn stop(&self) { self.running.store(false, Ordering::Relaxed); }
}

impl Drop for InfluxWriter { fn drop(&mut self) { self.running.store(false, Ordering::Relaxed); } }
