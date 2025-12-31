use crate::models::alerts::{AlertSeverity, Anomaly};
use crate::models::metrics::SystemMetrics;
use chrono::Utc;
use std::collections::HashMap;

struct Threshold { warn: f64, crit: f64 }

pub struct AnomalyDetector { th: HashMap<String, Threshold> }

impl AnomalyDetector {
    pub fn new() -> Self {
        Self { th: HashMap::from([
            ("cpu_usage".into(), Threshold { warn: 70.0, crit: 90.0 }),
            ("memory_percent".into(), Threshold { warn: 80.0, crit: 95.0 }),
            ("disk_usage".into(), Threshold { warn: 80.0, crit: 95.0 }),
            ("temperature".into(), Threshold { warn: 70.0, crit: 85.0 }),
            ("network_errors".into(), Threshold { warn: 100.0, crit: 1000.0 }),
            ("load_average".into(), Threshold { warn: 4.0, crit: 8.0 }),
        ]) }
    }

    pub fn analyze_metrics(&self, m: &SystemMetrics) -> Vec<Anomaly> {
        let mut out = Vec::new();
        let now = Utc::now();

        if !m.hardware.cpu_usage.is_empty() {
            let avg: f64 = m.hardware.cpu_usage.iter().sum::<f64>() / m.hardware.cpu_usage.len() as f64;
            if let Some(a) = self.check(m, "cpu_usage", avg, &now) { out.push(a); }
        }

        let mem = &m.hardware.memory_usage;
        if mem.total > 0 {
            let pct = (mem.used as f64 / mem.total as f64) * 100.0;
            if let Some(a) = self.check(m, "memory_percent", pct, &now) { out.push(a); }
        }

        for s in &m.hardware.thermal_sensors {
            if let Some(th) = self.th.get("temperature") {
                if let Some(sev) = self.severity(s.temperature, th) {
                    out.push(Anomaly {
                        node_id: m.node_id.clone(), timestamp: m.timestamp,
                        metric_name: format!("temperature_{}", s.name), value: s.temperature, severity: sev,
                        threshold_warning: Some(th.warn), threshold_critical: Some(th.crit), detected_at: now,
                        metadata: HashMap::from([("sensor_name".into(), serde_json::Value::String(s.name.clone()))]),
                    });
                }
            }
        }

        for fs in &m.storage.filesystems {
            if let Some(th) = self.th.get("disk_usage") {
                if let Some(sev) = self.severity(fs.usage_percent, th) {
                    out.push(Anomaly {
                        node_id: m.node_id.clone(), timestamp: m.timestamp,
                        metric_name: format!("disk_usage_{}", fs.mount_point.replace('/', "_")), value: fs.usage_percent, severity: sev,
                        threshold_warning: Some(th.warn), threshold_critical: Some(th.crit), detected_at: now,
                        metadata: HashMap::from([("mount_point".into(), serde_json::Value::String(fs.mount_point.clone()))]),
                    });
                }
            }
        }

        for iface in &m.network.interfaces {
            let errs = (iface.errors_in + iface.errors_out) as f64;
            if let Some(th) = self.th.get("network_errors") {
                if let Some(sev) = self.severity(errs, th) {
                    out.push(Anomaly {
                        node_id: m.node_id.clone(), timestamp: m.timestamp,
                        metric_name: format!("network_errors_{}", iface.name), value: errs, severity: sev,
                        threshold_warning: Some(th.warn), threshold_critical: Some(th.crit), detected_at: now,
                        metadata: HashMap::from([("interface".into(), serde_json::Value::String(iface.name.clone()))]),
                    });
                }
            }
        }
        out
    }

    fn check(&self, m: &SystemMetrics, name: &str, val: f64, now: &chrono::DateTime<Utc>) -> Option<Anomaly> {
        let th = self.th.get(name)?;
        let sev = self.severity(val, th)?;
        Some(Anomaly {
            node_id: m.node_id.clone(), timestamp: m.timestamp,
            metric_name: name.to_string(), value: val, severity: sev,
            threshold_warning: Some(th.warn), threshold_critical: Some(th.crit),
            detected_at: *now, metadata: HashMap::new(),
        })
    }

    fn severity(&self, val: f64, th: &Threshold) -> Option<AlertSeverity> {
        if val >= th.crit { Some(AlertSeverity::Critical) }
        else if val >= th.warn { Some(AlertSeverity::Warning) }
        else { None }
    }
}

impl Default for AnomalyDetector { fn default() -> Self { Self::new() } }
