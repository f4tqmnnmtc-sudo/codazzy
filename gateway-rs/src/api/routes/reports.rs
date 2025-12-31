use crate::api::routes::AppState;
use crate::config::CONFIG;
use crate::AppError;
use axum::{
    body::Body,
    extract::{Path, Query, State},
    http::{header, StatusCode},
    response::Response,
    Json,
};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportRequest {
    pub report_type: String,
    pub report_config: ReportConfig,
    pub data: ReportData,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportConfig {
    pub title: Option<String>,
    pub time_range: Option<String>,
    pub language: Option<String>,
    pub format: Option<String>,
    pub custom_prompt: Option<String>,
    pub detection_method: Option<String>,
    pub sensitivity: Option<String>,
    pub custom_thresholds: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Deserialize)]
pub struct ReportData {
    pub servers: Option<Vec<ServerInfo>>,
    pub anomalies: Option<Vec<Anomaly>>,
    pub predictions: Option<Vec<Prediction>>,
    pub metrics: Option<Vec<MetricData>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct ServerInfo {
    pub id: Option<String>,
    pub node_id: Option<String>,
    pub name: Option<String>,
    #[serde(rename = "type")]
    pub server_type: Option<String>,
    pub status: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Anomaly {
    pub metric_name: Option<String>,
    pub metric_type: Option<String>,
    pub node_id: Option<String>,
    pub server_id: Option<String>,
    pub severity: Option<String>,
    pub value: Option<f64>,
    pub threshold: Option<f64>,
    pub threshold_critical: Option<f64>,
    pub timestamp: Option<String>,
    pub mount_point: Option<String>,
    pub interface: Option<String>,
    pub sensor: Option<String>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct Prediction {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub metric_type: Option<String>,
    pub model_type: Option<String>,
    pub count: Option<i32>,
    pub avg_predicted: Option<f64>,
    pub min_predicted: Option<f64>,
    pub max_predicted: Option<f64>,
    pub first_timestamp: Option<String>,
    pub last_timestamp: Option<String>,
    pub prediction: Option<String>,
    pub confidence: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct MetricData {
    pub server_id: Option<String>,
    pub server_name: Option<String>,
    pub metric_type: Option<String>,
    pub count: Option<i32>,
    pub avg: Option<f64>,
    pub min: Option<f64>,
    pub max: Option<f64>,
    pub last_value: Option<f64>,
    pub first_time: Option<String>,
    pub last_time: Option<String>,
    pub data: Option<Vec<DataPoint>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct DataPoint {
    pub value: Option<f64>,
    pub timestamp: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportResponse {
    pub content: String,
    pub metadata: ReportMetadata,
    pub visual_data: Option<VisualData>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub debug_prompt: Option<DebugPrompt>,
}

#[derive(Debug, Serialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct DebugPrompt {
    pub system_prompt: String,
    pub user_prompt: String,
    pub model: String,
    pub timestamp: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ReportMetadata {
    pub report_type: String,
    pub generated_at: String,
    pub servers_count: usize,
    pub anomalies_count: usize,
    pub predictions_count: usize,
    pub language: String,
    pub format: String,
    pub model: String,
}

#[derive(Debug, Serialize, Default)]
#[serde(rename_all = "camelCase")]
pub struct VisualData {
    pub performance_trend: Vec<PerformancePoint>,
    pub resource_distribution: Vec<ResourceDistribution>,
    pub anomaly_trend: Vec<AnomalyTrend>,
    pub kpis: HashMap<String, f64>,
    pub top_processes: Vec<ProcessInfo>,
    pub detected_services: Vec<ServiceInfo>,
}

#[derive(Debug, Serialize)]
pub struct PerformancePoint {
    pub time: String,
    pub cpu: f64,
    pub memoria: f64,
    pub disco: f64,
}

#[derive(Debug, Serialize)]
pub struct ResourceDistribution {
    pub name: String,
    pub value: f64,
    pub color: String,
}

#[derive(Debug, Serialize)]
pub struct AnomalyTrend {
    pub fecha: String,
    pub criticas: i32,
    pub warnings: i32,
    pub info: i32,
}

#[derive(Debug, Serialize)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu: f64,
    pub memory_mb: f64,
}

#[derive(Debug, Serialize)]
pub struct ServiceInfo {
    pub name: String,
    pub display_name: String,
    pub status: String,
    pub process_count: i32,
}

#[derive(Debug, Deserialize)]
pub struct ExportQuery {
    pub format: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ExportRequest {
    pub title: Option<String>,
    pub content: Option<String>,
    pub metadata: Option<HashMap<String, serde_json::Value>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReportTemplate {
    pub name: String,
    #[serde(default)]
    pub description: String,
    pub system_prompt: String,
    pub user_prompt_template: String,
}

#[derive(Debug, Deserialize)]
struct TemplatesFile {
    templates: HashMap<String, ReportTemplate>,
}

const TEMPLATES_YAML: &str = include_str!("report_templates.yaml");

static REPORT_TEMPLATES: Lazy<HashMap<String, ReportTemplate>> = Lazy::new(|| {
    match serde_yaml::from_str::<TemplatesFile>(TEMPLATES_YAML) {
        Ok(file) => file.templates,
        Err(e) => {
            tracing::error!("Error cargando plantillas de reportes: {e}");
            HashMap::new()
        }
    }
});

fn format_servers_section(servers: &[ServerInfo]) -> String {
    if servers.is_empty() {
        return "No hay servidores especificados".into();
    }

    // Separamos por tipo: SNMP (red) vs agentes
    let (net_devices, agent_servers): (Vec<_>, Vec<_>) = servers.iter().partition(|srv| {
        let stype = srv.server_type.as_deref().unwrap_or("");
        let sname = srv.name.as_deref().or(srv.node_id.as_deref()).unwrap_or("");

        stype == "network_device"
            || sname.contains("-clab")
            || sname.contains("router")
            || sname.starts_with("sw")
    });

    let mut out = String::with_capacity(512);

    if !net_devices.is_empty() {
        out.push_str("### Dispositivos de Red (SNMP)\n");
        for dev in &net_devices {
            let name = dev.name.as_deref().or(dev.node_id.as_deref()).unwrap_or("Unknown");
            let status = dev.status.as_deref().unwrap_or("unknown");
            out.push_str(&format!("- **{}** - Estado: {} (monitorizado vía SNMP)\n", name, status));
        }
    }

    if !agent_servers.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### Servidores con Agente\n");
        for srv in &agent_servers {
            let name = srv.name.as_deref().or(srv.node_id.as_deref()).unwrap_or("Unknown");
            let status = srv.status.as_deref().unwrap_or("unknown");
            out.push_str(&format!("- **{}** - Estado: {} (agente instalado)\n", name, status));
        }
    }

    if out.is_empty() {
        "No hay dispositivos especificados".into()
    } else {
        out.trim_end().into()
    }
}

/// Genera un resumen agrupado por severidad
fn format_anomaly_summary(anomalies: &[Anomaly]) -> String {
    if anomalies.is_empty() {
        return "No se detectaron anomalías en el período analizado".into();
    }

    let mut by_severity: HashMap<&str, usize> = HashMap::new();
    for a in anomalies {
        let sev = a.severity.as_deref().unwrap_or("unknown");
        *by_severity.entry(sev).or_default() += 1;
    }

    // Orden fijo de severidades para consistencia
    let severity_order = ["critical", "high", "moderate", "low"];
    severity_order
        .iter()
        .filter_map(|&sev| {
            by_severity.get(sev).map(|count| {
                format!("- **{}**: {} anomalías", sev.to_uppercase(), count)
            })
        })
        .collect::<Vec<_>>()
        .join("\n")
}

/// Detalle técnico de cada anomalía (máx 20 para no saturar el prompt)
fn format_anomaly_details(anomalies: &[Anomaly]) -> String {
    if anomalies.is_empty() {
        return "No se detectaron anomalías técnicas".into();
    }

    let mut lines = Vec::with_capacity(anomalies.len().min(20));

    for anom in anomalies.iter().take(20) {
        let metric = anom.metric_name.as_deref()
            .or(anom.metric_type.as_deref())
            .unwrap_or("Unknown");
        let node = anom.node_id.as_deref()
            .or(anom.server_id.as_deref())
            .unwrap_or("unknown");
        let sev = anom.severity.as_deref().unwrap_or("unknown");
        let val = anom.value.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "N/A".into());
        let thresh = anom.threshold_critical.or(anom.threshold)
            .map(|t| format!("{:.2}", t))
            .unwrap_or_else(|| "N/A".into());
        let ts = anom.timestamp.as_deref().unwrap_or("unknown");

        // Info adicional si existe
        let mut extras = Vec::new();
        if let Some(mp) = &anom.mount_point {
            extras.push(format!("Punto de montaje: {}", mp));
        }
        if let Some(iface) = &anom.interface {
            extras.push(format!("Interfaz: {}", iface));
        }
        if let Some(sensor) = &anom.sensor {
            extras.push(format!("Sensor: {}", sensor));
        }

        let extra_line = if extras.is_empty() {
            String::new()
        } else {
            format!("\n- {}", extras.join(", "))
        };

        lines.push(format!(
            "\n**{}** - Severidad: {}\n- Servidor: {}\n- Timestamp: {}\n- Valor: {}\n- Umbral Crítico: {}{}",
            metric.to_uppercase(), sev.to_uppercase(), node, ts, val, thresh, extra_line
        ));
    }

    lines.join("\n")
}

fn format_predictions(preds: &[Prediction]) -> String {
    if preds.is_empty() {
        return "No hay predicciones disponibles".into();
    }

    preds.iter().map(|p| {
        let srv = p.server_name.as_deref().unwrap_or("Unknown");
        let metric = p.metric_type.as_deref().unwrap_or("Unknown");
        let model = p.model_type.as_deref().unwrap_or("chronos");
        let n = p.count.unwrap_or(0);
        let conf = p.confidence.unwrap_or(0.9) * 100.0;

        if n > 0 {
            format!(
                "- **{} - {}** (Modelo: {})\n  - Predicciones: {} puntos\n  - Promedio: {:.2}%\n  - Rango: {:.2}% - {:.2}%\n  - Confianza: {:.1}%",
                srv, metric, model, n,
                p.avg_predicted.unwrap_or(0.0),
                p.min_predicted.unwrap_or(0.0),
                p.max_predicted.unwrap_or(0.0),
                conf
            )
        } else {
            let pred_text = p.prediction.as_deref().unwrap_or("N/A");
            format!("- **{}**: {} (Confianza: {:.1}%)", metric, pred_text, conf)
        }
    }).collect::<Vec<_>>().join("\n")
}

/// Formatea métricas separando agentes de dispositivos de red
fn format_metrics_section(metrics: &[MetricData]) -> String {
    if metrics.is_empty() {
        return "No hay métricas disponibles".into();
    }

    // Heurística: CPU/memory/disk/load/temp son de agente, el resto es red
    let (agent_metrics, net_metrics): (Vec<_>, Vec<_>) = metrics.iter().partition(|m| {
        let mtype = m.metric_type.as_deref().unwrap_or("").to_lowercase();
        mtype == "cpu" || mtype == "memory" || mtype == "disk"
            || mtype.contains("load") || mtype.contains("temperature")
    });

    let mut out = String::with_capacity(1024);

    if !agent_metrics.is_empty() {
        out.push_str("### Métricas de Servidores (Agentes)\n");
        for m in &agent_metrics {
            if let Some(line) = format_single_metric(m) {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }

    if !net_metrics.is_empty() {
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str("### Métricas de Red (Throughput)\n");
        for m in &net_metrics {
            if let Some(line) = format_single_metric(m) {
                out.push_str(&line);
                out.push('\n');
            }
        }
    }

    if out.is_empty() {
        "No hay métricas disponibles".into()
    } else {
        out.trim_end().into()
    }
}

fn format_single_metric(m: &MetricData) -> Option<String> {
    let srv = m.server_name.as_deref().unwrap_or("Unknown");
    let mtype = m.metric_type.as_deref().unwrap_or("Unknown");

    // Si tenemos estadísticas agregadas
    if let (Some(avg), Some(min), Some(max)) = (m.avg, m.min, m.max) {
        let unit = get_metric_unit(mtype);
        let samples = m.count.unwrap_or(0);
        let last = m.last_value.unwrap_or(avg);

        return Some(format!(
            "- **{} - {}**:\n  - Promedio: {:.2}{}\n  - Mínimo: {:.2}{}\n  - Máximo: {:.2}{}\n  - Último valor: {:.2}{}\n  - Muestras: {}",
            srv, mtype, avg, unit, min, unit, max, unit, last, unit, samples
        ));
    }

    // Fallback: calcular promedio de datapoints crudos
    m.data.as_ref().and_then(|points| {
        let vals: Vec<f64> = points.iter().filter_map(|p| p.value).collect();
        if vals.is_empty() {
            return None;
        }
        let avg = vals.iter().sum::<f64>() / vals.len() as f64;
        Some(format!("- **{} - {}**: Promedio: {:.2}", srv, mtype, avg))
    })
}

fn get_metric_unit(mtype: &str) -> &'static str {
    let lower = mtype.to_lowercase();

    // Throughput de red
    if lower.contains("bytes_in") || lower.contains("bytes_out") {
        return " Mbps";
    }

    match lower.as_str() {
        "cpu" | "cpu_usage" => "%",
        "memory" | "memory_percent" => "%",
        "memory_used" => " GB",
        "disk" | "disk_percent" | "disk_usage" => "%",
        "network" | "network_bytes_sent" | "network_bytes_recv" => " Mbps",
        "temperature" => "°C",
        "load_1" | "load_5" | "load_15" => "",
        _ => "",
    }
}


fn build_visual_data() -> VisualData {
    // TODO: Esto debería calcularse dinámicamente de las métricas reales
    // Por ahora devolvemos datos de ejemplo para el frontend
    let week_days = ["Lun", "Mar", "Mié", "Jue", "Vie", "Sáb", "Dom"];
    let anomaly_data = [
        (0, 2, 5), (1, 1, 3), (0, 3, 4), (0, 0, 2),
        (0, 1, 3), (0, 0, 1), (0, 0, 0),
    ];

    VisualData {
        performance_trend: vec![],
        top_processes: vec![],
        detected_services: vec![],
        resource_distribution: vec![
            ResourceDistribution { name: "CPU".into(), value: 45.0, color: "#3b82f6".into() },
            ResourceDistribution { name: "Memoria".into(), value: 60.0, color: "#10b981".into() },
            ResourceDistribution { name: "Disco".into(), value: 35.0, color: "#f59e0b".into() },
            ResourceDistribution { name: "Red".into(), value: 15.0, color: "#06b6d4".into() },
        ],
        anomaly_trend: week_days.iter().zip(anomaly_data.iter())
            .map(|(day, &(crit, warn, info))| AnomalyTrend {
                fecha: day.to_string(),
                criticas: crit,
                warnings: warn,
                info,
            })
            .collect(),
        kpis: HashMap::from([
            ("cpu_avg".into(), 45.0),
            ("memory_avg".into(), 60.0),
        ]),
    }
}


pub async fn generate_report(
    State(state): State<AppState>,
    Json(req): Json<ReportRequest>,
) -> Result<Json<ReportResponse>, AppError> {
    // Validaciones iniciales
    let api_key = CONFIG.openai_api_key.as_ref()
        .ok_or_else(|| AppError::Configuration("OpenAI API key no configurada".into()))?;

    let tpl = REPORT_TEMPLATES.get(&req.report_type).ok_or_else(|| {
        let valid_types: Vec<_> = REPORT_TEMPLATES.keys().collect();
        AppError::Validation(format!(
            "Tipo de reporte '{}' no valido. Tipos soportados: {:?}",
            req.report_type, valid_types
        ))
    })?;

    let servers = req.data.servers.as_deref().unwrap_or(&[]);
    let anomalies = req.data.anomalies.as_deref().unwrap_or(&[]);
    let predictions = req.data.predictions.as_deref().unwrap_or(&[]);
    let metrics = req.data.metrics.as_deref().unwrap_or(&[]);
    let time_range = req.report_config.time_range.as_deref().unwrap_or("24h");

    let mut docs_ctx = String::new();
    let mut procs_ctx = String::new();
    let mut hw_ctx = String::new();

    for srv in servers {
        let sid = match srv.id.as_deref().or(srv.node_id.as_deref()) {
            Some(id) if !id.is_empty() => id,
            _ => continue,
        };

        if let Ok(docs) = state.server_documents_service.summaries_for_ai(sid).await {
            if !docs.is_empty() {
                docs_ctx.push_str(&format!("\n### Documentación de {} ###\n", sid));
                for doc in docs {
                    let fname = doc["filename"].as_str().unwrap_or("unknown");
                    let summary = doc["summary"].as_str().unwrap_or("");
                    if !summary.is_empty() {
                        docs_ctx.push_str(&format!("**{}**:\n{}\n\n", fname, summary));
                    }
                }
            }
        }

        if let Ok(procs) = state.server_processes_service.processes_for_ai(sid).await {
            if !procs.is_empty() {
                procs_ctx.push_str(&format!(
                    "\n### Procesos Activos en {} ###\n| Proceso | PID | CPU% | Memoria% | Estado |\n|---------|-----|------|----------|--------|\n",
                    sid
                ));
                for p in procs.iter().take(10) {
                    procs_ctx.push_str(&format!(
                        "| {} | {} | {:.1}% | {:.1}% | {} |\n",
                        p.process_name,
                        p.pid.unwrap_or(0),
                        p.cpu_usage.unwrap_or(0.0),
                        p.memory_percent.unwrap_or(0.0),
                        p.status.as_deref().unwrap_or("unknown")
                    ));
                }
                procs_ctx.push('\n');
            }
        }

        // Métricas de hardware en tiempo real
        if let Ok(hw) = state.influx_service.node_metrics(sid, "1h").await {
            if let Some(latest) = hw.get("latest").and_then(|v| v.as_object()) {
                hw_ctx.push_str(&format!("\n### Hardware de {} ###\n", sid));

                if let Some(cpu) = latest.get("cpu_percent").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **CPU Total:** {:.1}%\n", cpu));
                }
                if let Some(mem) = latest.get("memory_percent").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **Memoria Usada:** {:.1}%\n", mem));
                }
                if let Some(mem_total) = latest.get("memory_total").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **Memoria Total:** {:.0} GB\n", mem_total / 1_073_741_824.0));
                }
                if let Some(disk) = latest.get("disk_percent").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **Disco Usado:** {:.1}%\n", disk));
                }
                if let Some(disk_total) = latest.get("disk_total").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **Disco Total:** {:.0} GB\n", disk_total / 1_073_741_824.0));
                }
                if let Some(load) = latest.get("load_avg_1").and_then(|v| v.as_f64()) {
                    hw_ctx.push_str(&format!("- **Load Average (1m):** {:.2}\n", load));
                }
                if let Some(cores) = latest.get("cpu_cores").and_then(|v| v.as_i64()) {
                    hw_ctx.push_str(&format!("- **CPU Cores:** {}\n", cores));
                }
                if let Some(uptime) = latest.get("uptime_seconds").and_then(|v| v.as_f64()) {
                    let days = (uptime / 86400.0) as i64;
                    let hours = ((uptime % 86400.0) / 3600.0) as i64;
                    hw_ctx.push_str(&format!("- **Uptime:** {} días, {} horas\n", days, hours));
                }
                hw_ctx.push('\n');
            }
        }
    }

    let user_prompt = tpl.user_prompt_template
        .replace("{servers_info}", &format_servers_section(servers))
        .replace("{time_range}", time_range)
        .replace("{anomalies_count}", &anomalies.len().to_string())
        .replace("{anomalies_summary}", &format_anomaly_summary(anomalies))
        .replace("{anomalies_detailed}", &format_anomaly_details(anomalies))
        .replace("{predictions_summary}", &format_predictions(predictions))
        .replace("{predictions_detailed}", &format_predictions(predictions))
        .replace("{metrics_summary}", &format_metrics_section(metrics))
        .replace("{metrics_detailed}", &format_metrics_section(metrics))
        .replace("{processes_context}", &procs_ctx)
        .replace("{hardware_context}", &hw_ctx);

    // Añadir contexto adicional si existe
    let mut additional_ctx = String::new();
    if !hw_ctx.is_empty() {
        additional_ctx.push_str("\n\n## ESTADO DE HARDWARE ACTUAL\nMétricas de recursos en tiempo real de los servidores:\n");
        additional_ctx.push_str(&hw_ctx);
    }
    if !procs_ctx.is_empty() {
        additional_ctx.push_str("\n\n## PROCESOS EN EJECUCIÓN\nTop 10 procesos por consumo de recursos en cada servidor:\n");
        additional_ctx.push_str(&procs_ctx);
    }
    if !docs_ctx.is_empty() {
        additional_ctx.push_str("\n\n## DOCUMENTACIÓN DE CONTEXTO\nResúmenes de documentación que proporcionan contexto sobre servicios y configuraciones:\n");
        additional_ctx.push_str(&docs_ctx);
    }

    let prompt_with_ctx = if additional_ctx.is_empty() {
        user_prompt
    } else {
        format!("{}\n{}", user_prompt, additional_ctx)
    };

    let final_prompt = match &req.report_config.custom_prompt {
        Some(custom) if !custom.is_empty() => {
            format!("{}\n\n**REQUISITOS ADICIONALES:**\n{}", prompt_with_ctx, custom)
        }
        _ => prompt_with_ctx,
    };

    let payload = serde_json::json!({
        "model": CONFIG.openai_model,
        "messages": [
            { "role": "system", "content": &tpl.system_prompt },
            { "role": "user", "content": &final_prompt }
        ],
        "max_completion_tokens": CONFIG.openai_max_tokens,
        "reasoning_effort": "low"
    });

    let http = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(120))
        .build()
        .map_err(|e| AppError::Internal(format!("Error creando cliente HTTP: {e}")))?;

    let resp = http
        .post("https://api.openai.com/v1/chat/completions")
        .header("Authorization", format!("Bearer {}", api_key))
        .header("Content-Type", "application/json")
        .json(&payload)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Error en peticion a OpenAI: {e}")))?;

    if !resp.status().is_success() {
        let err_body = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("OpenAI API error: {}", err_body)));
    }

    let json_resp: serde_json::Value = resp.json().await
        .map_err(|e| AppError::Internal(format!("Error parseando respuesta de OpenAI: {e}")))?;

    let content = json_resp["choices"][0]["message"]["content"]
        .as_str()
        .ok_or_else(|| AppError::Internal("Respuesta vacia de OpenAI".into()))?
        .to_string();

    let now = chrono::Utc::now().to_rfc3339();

    Ok(Json(ReportResponse {
        content,
        visual_data: Some(build_visual_data()),
        metadata: ReportMetadata {
            report_type: req.report_type.clone(),
            generated_at: now.clone(),
            servers_count: servers.len(),
            anomalies_count: anomalies.len(),
            predictions_count: predictions.len(),
            language: req.report_config.language.clone().unwrap_or_else(|| "es".into()),
            format: req.report_config.format.clone().unwrap_or_else(|| "markdown".into()),
            model: CONFIG.openai_model.clone(),
        },
        debug_prompt: Some(DebugPrompt {
            system_prompt: tpl.system_prompt.clone(),
            user_prompt: final_prompt,
            model: CONFIG.openai_model.clone(),
            timestamp: now,
        }),
    }))
}

pub async fn export_report(
    Path(_report_id): Path<String>,
    Query(query): Query<ExportQuery>,
    Json(req): Json<ExportRequest>,
) -> Result<Response, AppError> {
    let fmt = query.format.as_deref().unwrap_or("pdf");
    let content = req.content.as_deref().unwrap_or("");
    let title = req.title.as_deref().unwrap_or("Informe");

    match fmt.to_lowercase().as_str() {
        "md" | "markdown" => {
            let safe_title: String = title.replace(' ', "_").chars().take(50).collect();
            let filename = format!("{}_{}.md", safe_title, chrono::Utc::now().format("%Y%m%d_%H%M%S"));

            Ok(Response::builder()
                .status(StatusCode::OK)
                .header(header::CONTENT_TYPE, "text/markdown; charset=utf-8")
                .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
                .body(Body::from(content.to_string()))
                .unwrap())
        }

        "pdf" => {
            let html = render_pdf_html(content, title);
            let safe_title: String = title.replace(' ', "_").chars().take(50).collect();
            let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");

            match generate_pdf(&html, title).await {
                Ok(pdf_bytes) => {
                    let filename = format!("{}_{}.pdf", safe_title, ts);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "application/pdf")
                        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
                        .body(Body::from(pdf_bytes))
                        .unwrap())
                }
                Err(e) => {
                    let filename = format!("{}_{}.html", safe_title, ts);
                    Ok(Response::builder()
                        .status(StatusCode::OK)
                        .header(header::CONTENT_TYPE, "text/html; charset=utf-8")
                        .header(header::CONTENT_DISPOSITION, format!("attachment; filename=\"{}\"", filename))
                        .header("X-PDF-Fallback", "true")
                        .header("X-PDF-Error", e.to_string())
                        .body(Body::from(html))
                        .unwrap())
                }
            }
        }

        _ => Err(AppError::Validation(format!(
            "Formato '{}' no valido. Formatos soportados: pdf, md, markdown",
            fmt
        ))),
    }
}

fn render_pdf_html(markdown_content: &str, title: &str) -> String {
    use pulldown_cmark::{html, Parser};

    let mut html_body = String::new();
    html::push_html(&mut html_body, Parser::new(markdown_content));

    // Colores según tipo de reporte
    let title_lower = title.to_lowercase();
    let (primary, accent, subtitle) = if title_lower.contains("ejecutivo") {
        ("#e85d04", "#f48c06", "Executive Report")
    } else if title_lower.contains("red") || title_lower.contains("network") {
        ("#d00000", "#dc2f02", "Network Analysis")
    } else {
        ("#e85d04", "#f48c06", "Technical Report")
    };

    let now = chrono::Local::now();
    let date_str = now.format("%d/%m/%Y").to_string();
    let time_str = now.format("%H:%M:%S").to_string();
    let report_id = now.format("%Y%m%d%H%M").to_string();

    format!(
        r##"<!DOCTYPE html>
<html lang="es">
<head>
    <meta charset="UTF-8">
    <title>{title}</title>
    <style>
        @import url('https://fonts.googleapis.com/css2?family=Inter:wght@400;500;600;700&display=swap');
        
        :root {{
            --primary: {primary};
            --accent: {accent};
            --orange: #e85d04;
            --red: #d00000;
            --yellow: #faa307;
            --text-dark: #2d3436;
            --text-body: #4a4a4a;
            --text-muted: #6c757d;
            --border: #dee2e6;
            --bg-light: #f8f9fa;
        }}
        
        * {{ margin: 0; padding: 0; box-sizing: border-box; }}
        
        body {{
            font-family: 'Inter', -apple-system, BlinkMacSystemFont, 'Segoe UI', Roboto, sans-serif;
            max-width: 210mm;
            margin: 0 auto;
            padding: 0;
            line-height: 1.7;
            color: var(--text-body);
            background: white;
            font-size: 11px;
            position: relative;
        }}
        
        .page-decoration {{
            position: absolute;
            top: 0;
            right: 0;
            width: 120px;
            height: 100%;
            overflow: hidden;
            z-index: 0;
        }}
        
        .deco-circle {{ position: absolute; border-radius: 50%; }}
        .deco-circle-1 {{ width: 80px; height: 80px; background: var(--yellow); top: 20px; right: 30px; opacity: 0.9; }}
        .deco-circle-2 {{ width: 40px; height: 40px; background: var(--orange); top: 90px; right: 80px; opacity: 0.8; }}
        
        .deco-lines {{ position: absolute; top: 150px; right: 20px; width: 60px; }}
        .deco-line {{ height: 4px; margin-bottom: 6px; border-radius: 2px; }}
        .deco-line-1 {{ background: var(--red); width: 100%; }}
        .deco-line-2 {{ background: var(--orange); width: 80%; }}
        .deco-line-3 {{ background: var(--yellow); width: 60%; }}
        .deco-line-4 {{ background: var(--red); width: 40%; }}
        
        .deco-dots {{
            position: absolute;
            bottom: 100px;
            right: 30px;
            display: grid;
            grid-template-columns: repeat(3, 8px);
            gap: 6px;
        }}
        .deco-dot {{ width: 8px; height: 8px; background: var(--orange); border-radius: 50%; opacity: 0.7; }}
        .deco-dot:nth-child(2), .deco-dot:nth-child(5), .deco-dot:nth-child(8) {{ background: var(--yellow); }}
        .deco-dot:nth-child(3), .deco-dot:nth-child(6) {{ background: var(--red); }}
        
        .left-border {{
            position: absolute;
            left: 0;
            top: 0;
            width: 6px;
            height: 100%;
            background: linear-gradient(180deg, var(--orange) 0%, var(--red) 50%, var(--yellow) 100%);
        }}
        
        .report-header {{
            padding: 30px 40px 25px 50px;
            position: relative;
            z-index: 1;
        }}
        
        .header-brand {{
            display: flex;
            align-items: center;
            gap: 12px;
            margin-bottom: 20px;
        }}
        
        .brand-icon {{
            width: 36px;
            height: 36px;
            background: linear-gradient(135deg, var(--orange), var(--red));
            border-radius: 8px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: 700;
            font-size: 18px;
        }}
        
        .brand-text {{
            font-size: 16px;
            font-weight: 700;
            color: var(--text-dark);
            letter-spacing: -0.3px;
        }}
        
        .report-title {{
            font-size: 26px;
            font-weight: 700;
            color: var(--primary);
            margin: 0 0 6px 0;
            letter-spacing: -0.5px;
            line-height: 1.2;
        }}
        
        .report-date {{
            font-size: 12px;
            color: var(--text-muted);
            margin-bottom: 20px;
        }}
        
        .info-box {{
            background: var(--bg-light);
            border-radius: 8px;
            padding: 16px 20px;
            margin-bottom: 10px;
            border-left: 4px solid var(--primary);
        }}
        
        .info-row {{
            display: flex;
            margin-bottom: 6px;
            font-size: 11px;
        }}
        .info-row:last-child {{ margin-bottom: 0; }}
        
        .info-label {{
            color: var(--text-muted);
            width: 140px;
            flex-shrink: 0;
        }}
        
        .info-value {{
            color: var(--text-dark);
            font-weight: 500;
        }}
        
        .content {{
            padding: 20px 40px 30px 50px;
            position: relative;
            z-index: 1;
        }}
        
        h1 {{
            color: var(--text-dark);
            font-size: 16px;
            font-weight: 700;
            margin: 28px 0 14px 0;
            padding-bottom: 8px;
            border-bottom: 2px solid var(--primary);
        }}
        h1:first-child {{ margin-top: 0; }}
        
        h2 {{
            color: var(--text-dark);
            font-size: 13px;
            font-weight: 600;
            margin: 22px 0 10px 0;
        }}
        
        h3 {{
            color: var(--text-body);
            font-size: 12px;
            font-weight: 600;
            margin: 16px 0 8px 0;
        }}
        
        p {{
            margin: 10px 0;
            color: var(--text-body);
            line-height: 1.7;
        }}
        
        ul, ol {{
            margin: 12px 0;
            padding-left: 0;
            list-style: none;
        }}
        
        li {{
            margin: 8px 0;
            padding-left: 20px;
            position: relative;
            color: var(--text-body);
            line-height: 1.6;
        }}
        
        li::before {{
            content: '';
            position: absolute;
            left: 0;
            top: 8px;
            width: 8px;
            height: 8px;
            background: var(--orange);
            border-radius: 50%;
        }}
        li:nth-child(3n+2)::before {{ background: var(--red); }}
        li:nth-child(3n)::before {{ background: var(--yellow); }}
        
        strong {{
            color: var(--text-dark);
            font-weight: 600;
        }}
        
        code {{
            background: var(--bg-light);
            color: var(--primary);
            padding: 2px 8px;
            border-radius: 4px;
            font-family: 'SF Mono', 'Consolas', monospace;
            font-size: 10px;
        }}
        
        pre {{
            background: #2d3436;
            color: #dfe6e9;
            padding: 16px 20px;
            border-radius: 8px;
            font-family: 'SF Mono', 'Consolas', monospace;
            font-size: 10px;
            overflow-x: auto;
            margin: 16px 0;
            border-left: 4px solid var(--primary);
        }}
        
        table {{
            border-collapse: collapse;
            width: 100%;
            margin: 18px 0;
            font-size: 10px;
            border-radius: 8px;
            overflow: hidden;
        }}
        
        th {{
            background: var(--primary);
            color: white;
            padding: 12px 14px;
            text-align: left;
            font-weight: 600;
            font-size: 10px;
        }}
        
        td {{
            border-bottom: 1px solid var(--border);
            padding: 11px 14px;
            color: var(--text-body);
        }}
        
        tr:nth-child(even) td {{ background: var(--bg-light); }}
        
        blockquote {{
            border-left: 4px solid var(--accent);
            padding: 14px 20px;
            margin: 16px 0;
            background: #fff8f0;
            border-radius: 0 8px 8px 0;
            color: var(--text-body);
        }}
        
        hr {{
            border: none;
            height: 1px;
            background: var(--border);
            margin: 24px 0;
        }}
        
        .report-footer {{
            margin-top: 30px;
            padding: 20px 40px 20px 50px;
            border-top: 2px solid var(--border);
            display: flex;
            justify-content: space-between;
            align-items: center;
            position: relative;
            z-index: 1;
        }}
        
        .footer-brand {{
            display: flex;
            align-items: center;
            gap: 10px;
        }}
        
        .footer-icon {{
            width: 28px;
            height: 28px;
            background: linear-gradient(135deg, var(--orange), var(--red));
            border-radius: 6px;
            display: flex;
            align-items: center;
            justify-content: center;
            color: white;
            font-weight: 700;
            font-size: 14px;
        }}
        
        .footer-text {{
            font-size: 11px;
            color: var(--text-muted);
        }}
        
        .footer-meta {{
            text-align: right;
            font-size: 10px;
            color: var(--text-muted);
        }}
        
        .notice {{
            margin: 0 40px 20px 50px;
            padding: 12px 16px;
            background: #fff3cd;
            border: 1px solid #ffc107;
            border-radius: 6px;
            font-size: 10px;
            color: #856404;
            position: relative;
            z-index: 1;
        }}
        
        /* Print styles */
        h1, h2, h3, h4 {{ page-break-after: avoid; break-after: avoid; }}
        p, li {{ orphans: 3; widows: 3; }}
        table {{ page-break-inside: avoid; break-inside: avoid; }}
        tr {{ page-break-inside: avoid; break-inside: avoid; }}
        thead {{ display: table-header-group; }}
        pre, blockquote {{ page-break-inside: avoid; break-inside: avoid; }}
        img {{ page-break-inside: avoid; break-inside: avoid; max-width: 100%; }}
        .report-header {{ page-break-after: avoid; break-after: avoid; }}
        .report-footer {{ page-break-before: avoid; break-before: avoid; }}
        
        @media print {{
            body {{ print-color-adjust: exact; -webkit-print-color-adjust: exact; }}
        }}
    </style>
</head>
<body>
    <div class="left-border"></div>
    <div class="page-decoration">
        <div class="deco-circle deco-circle-1"></div>
        <div class="deco-circle deco-circle-2"></div>
        <div class="deco-lines">
            <div class="deco-line deco-line-1"></div>
            <div class="deco-line deco-line-2"></div>
            <div class="deco-line deco-line-3"></div>
            <div class="deco-line deco-line-4"></div>
        </div>
        <div class="deco-dots">
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
            <div class="deco-dot"></div>
        </div>
    </div>
    
    <div class="report-header">
        <div class="header-brand">
            <div class="brand-icon">C</div>
            <div class="brand-text">Codazzy</div>
        </div>
        <h1 class="report-title">{title}</h1>
        <div class="report-date">Fecha: {date}</div>
        <div class="info-box">
            <div class="info-row">
                <span class="info-label">Tipo de Informe:</span>
                <span class="info-value">{subtitle}</span>
            </div>
            <div class="info-row">
                <span class="info-label">Generado:</span>
                <span class="info-value">{date} - {time}</span>
            </div>
            <div class="info-row">
                <span class="info-label">ID de Reporte:</span>
                <span class="info-value">RPT-{rid}</span>
            </div>
            <div class="info-row">
                <span class="info-label">Sistema:</span>
                <span class="info-value">Codazzy Infrastructure Monitoring</span>
            </div>
        </div>
    </div>
    
    <div class="content">{body}</div>
    
    <div class="report-footer">
        <div class="footer-brand">
            <div class="footer-icon">C</div>
            <div class="footer-text">Codazzy Infrastructure Monitoring Platform</div>
        </div>
        <div class="footer-meta">
            <div>Documento generado automáticamente</div>
            <div>{date} {time}</div>
        </div>
    </div>
    
    <div class="notice">
        <strong>Aviso:</strong> Este documento contiene información de infraestructura que puede ser confidencial. Distribúyalo únicamente a personal autorizado.
    </div>
</body>
</html>"##,
        title = title,
        primary = primary,
        accent = accent,
        subtitle = subtitle,
        date = date_str,
        time = time_str,
        rid = report_id,
        body = html_body
    )
}

async fn try_pdf_tool(
    tool: &str,
    html_path: &std::path::Path,
    pdf_path: &std::path::Path,
    title: &str,
) -> Option<Vec<u8>> {
    use tokio::process::Command;

    let result = match tool {
        "wkhtmltopdf" => {
            Command::new("wkhtmltopdf")
                .args([
                    "--quiet",
                    "--encoding", "utf-8",
                    "--page-size", "A4",
                    "--margin-top", "20mm",
                    "--margin-bottom", "20mm",
                    "--margin-left", "15mm",
                    "--margin-right", "15mm",
                    "--title", title,
                ])
                .arg(html_path)
                .arg(pdf_path)
                .output()
                .await
        }
        "weasyprint" => {
            Command::new("weasyprint")
                .arg(html_path)
                .arg(pdf_path)
                .output()
                .await
        }
        _ => return None,
    };

    match result {
        Ok(output) if output.status.success() => tokio::fs::read(pdf_path).await.ok(),
        _ => None,
    }
}

async fn generate_pdf(html: &str, title: &str) -> Result<Vec<u8>, AppError> {
    let tmp = std::env::temp_dir();
    let uid = uuid::Uuid::new_v4();
    let html_path = tmp.join(format!("report_{}.html", uid));
    let pdf_path = tmp.join(format!("report_{}.pdf", uid));

    tokio::fs::write(&html_path, html)
        .await
        .map_err(|e| AppError::Internal(format!("Error escribiendo HTML temporal: {e}")))?;

    for tool in ["wkhtmltopdf", "weasyprint"] {
        if let Some(pdf_bytes) = try_pdf_tool(tool, &html_path, &pdf_path, title).await {
            // Limpiar archivos temporales
            let _ = tokio::fs::remove_file(&html_path).await;
            let _ = tokio::fs::remove_file(&pdf_path).await;
            return Ok(pdf_bytes);
        }
    }

    let _ = tokio::fs::remove_file(&html_path).await;
    let _ = tokio::fs::remove_file(&pdf_path).await;

    Err(AppError::Internal(
        "Generacion de PDF requiere wkhtmltopdf o weasyprint. Instalar con: sudo apt install wkhtmltopdf".into()
    ))
}
