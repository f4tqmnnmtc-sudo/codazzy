use crate::config::CONFIG;
use crate::error::{AppError, Result};
use crate::services::server_processes_service::ServerProcessesService;
use crate::services::ServerDocumentsService;
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use std::sync::Arc;

const PROMPT_TEMPLATE: &str = include_str!("../../templates/threshold_analysis.txt");
const SYSTEM_PROMPT: &str = "Eres un experto en monitorizacion de infraestructura IT. Responde SIEMPRE en formato JSON valido.";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnalyzeDeviceRequest {
    pub device_id: String,
    pub device_name: String,
    pub device_type: String,
    #[serde(default = "default_protocol")]
    pub protocol: String,
    pub available_metrics: Vec<String>,
    pub location: Option<String>,
    pub processes: Option<Vec<ProcessInfo>>,
    pub current_metrics: Option<CurrentMetrics>,
}

fn default_protocol() -> String { "agent".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProcessInfo {
    pub name: String,
    pub cpu_usage: f32,
    pub memory_bytes: u64,
    pub memory_percent: f32,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrentMetrics {
    pub cpu_percent: Option<f64>,
    pub memory_percent: Option<f64>,
    pub disk_percent: Option<f64>,
    pub load_1: Option<f64>,
    pub load_5: Option<f64>,
    pub load_15: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdResult {
    pub device_id: String,
    pub device_type_detected: String,
    pub thresholds_created: usize,
    pub thresholds: Vec<ThresholdCfg>,
    pub ignored_metrics: Vec<String>,
    pub general_notes: String,
    pub ai_model: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ThresholdCfg {
    pub metric: String,
    pub display_name: String,
    pub unit: String,
    pub warning: f64,
    pub critical: f64,
    pub comparison: String,
    pub priority: String,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
struct GptResp {
    device_type_detected: Option<String>,
    thresholds: Option<Vec<GptThreshold>>,
    ignored_metrics: Option<Vec<String>>,
    general_notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct GptThreshold {
    metric: String,
    display_name: Option<String>,
    unit: Option<String>,
    warning: f64,
    critical: f64,
    comparison: Option<String>,
    priority: Option<String>,
    reason: Option<String>,
}

pub struct ThresholdAIService {
    pool: PgPool,
    procs_svc: Arc<ServerProcessesService>,
    docs_svc: Arc<ServerDocumentsService>,
}

impl ThresholdAIService {
    pub fn new(
        pool: PgPool,
        procs_svc: Arc<ServerProcessesService>,
        docs_svc: Arc<ServerDocumentsService>,
    ) -> Self {
        Self { pool, procs_svc, docs_svc }
    }

    pub async fn analyze(&self, mut req: AnalyzeDeviceRequest) -> Result<ThresholdResult> {
        self.enrich_processes(&mut req).await;
        let docs_ctx = self.fetch_docs_context(&req.device_id).await;
        
        let api_key = CONFIG.openai_api_key.as_ref()
            .ok_or_else(|| AppError::Configuration("OpenAI API key no configurada".into()))?;

        let prompt = self.build_prompt(&req, &docs_ctx);
        let gpt_resp = self.call_gpt(api_key, &prompt).await
            .unwrap_or_else(|_| self.fallback(&req.device_type));

        let thresholds = gpt_resp.thresholds.clone().unwrap_or_default();
        let saved = self.persist(&req.device_id, &thresholds).await?;

        Ok(ThresholdResult {
            device_id: req.device_id,
            device_type_detected: gpt_resp.device_type_detected.unwrap_or(req.device_type),
            thresholds_created: saved,
            thresholds: thresholds.into_iter().map(|t| ThresholdCfg {
                metric: t.metric,
                display_name: t.display_name.unwrap_or_default(),
                unit: t.unit.unwrap_or_else(|| "%".into()),
                warning: t.warning,
                critical: t.critical,
                comparison: t.comparison.unwrap_or_else(|| "gt".into()),
                priority: t.priority.unwrap_or_else(|| "medium".into()),
                reason: t.reason.unwrap_or_default(),
            }).collect(),
            ignored_metrics: gpt_resp.ignored_metrics.unwrap_or_default(),
            general_notes: gpt_resp.general_notes.unwrap_or_default(),
            ai_model: CONFIG.openai_model.clone(),
        })
    }

    async fn enrich_processes(&self, req: &mut AnalyzeDeviceRequest) {
        if req.processes.as_ref().map(|p| p.is_empty()).unwrap_or(true) {
            if let Ok(stored) = self.procs_svc.processes_for_ai(&req.device_id).await {
                if !stored.is_empty() {
                    req.processes = Some(stored.iter().map(|p| ProcessInfo {
                        name: p.process_name.clone(),
                        cpu_usage: p.cpu_usage.unwrap_or(0.0),
                        memory_bytes: p.memory_bytes.unwrap_or(0) as u64,
                        memory_percent: p.memory_percent.unwrap_or(0.0),
                        status: p.status.clone().unwrap_or_else(|| "unknown".into()),
                    }).collect());
                }
            }
        }
    }

    async fn fetch_docs_context(&self, device_id: &str) -> String {
        self.docs_svc.documents(device_id).await
            .map(|docs| docs.iter()
                .map(|d| format!("### {} ({})\n{}", d.filename, d.description.as_deref().unwrap_or("Sin descripcion"), d.content))
                .collect::<Vec<_>>()
                .join("\n\n"))
            .unwrap_or_default()
    }

    fn build_prompt(&self, req: &AnalyzeDeviceRequest, docs_ctx: &str) -> String {
        let metrics = req.available_metrics.iter().take(50)
            .map(|m| format!("  - {m}"))
            .collect::<Vec<_>>()
            .join("\n");

        let procs = req.processes.as_ref()
            .map(|ps| ps.iter().take(15)
                .map(|p| format!("  - {} (CPU: {:.1}%, Mem: {:.1}%)", p.name, p.cpu_usage, p.memory_percent))
                .collect::<Vec<_>>()
                .join("\n"))
            .filter(|s| !s.is_empty())
            .unwrap_or_else(|| "No hay informacion de procesos".into());

        let curr = req.current_metrics.as_ref().map(|m| {
            let mut info = Vec::new();
            if let Some(v) = m.cpu_percent { info.push(format!("  - CPU actual: {v:.1}%")); }
            if let Some(v) = m.memory_percent { info.push(format!("  - Memoria actual: {v:.1}%")); }
            if let Some(v) = m.disk_percent { info.push(format!("  - Disco actual: {v:.1}%")); }
            if let Some(v) = m.load_1 { info.push(format!("  - Load 1m: {v:.2}")); }
            if info.is_empty() { "No hay metricas actuales disponibles".into() } else { info.join("\n") }
        }).unwrap_or_else(|| "No hay metricas actuales disponibles".into());

        let docs_section = if docs_ctx.is_empty() { String::new() }
            else { format!("\n## DOCUMENTACION DEL SERVIDOR\n{docs_ctx}\n") };

        PROMPT_TEMPLATE
            .replace("{device_id}", &req.device_id)
            .replace("{device_name}", &req.device_name)
            .replace("{device_type}", &req.device_type)
            .replace("{protocol}", &req.protocol)
            .replace("{location}", req.location.as_deref().unwrap_or("No especificada"))
            .replace("{metrics_list}", &metrics)
            .replace("{processes_info}", &procs)
            .replace("{current_metrics_info}", &curr)
            .replace("{documents_section}", &docs_section)
    }

    async fn call_gpt(&self, api_key: &str, prompt: &str) -> Result<GptResp> {
        let body = serde_json::json!({
            "model": CONFIG.openai_model,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": prompt }
            ],
            "max_completion_tokens": 4000,
            "reasoning_effort": "minimal"
        });

        let resp = reqwest::Client::new()
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {api_key}"))
            .header("Content-Type", "application/json")
            .json(&body)
            .send().await
            .map_err(|e| AppError::Internal(format!("OpenAI API fallo: {e}")))?;

        if !resp.status().is_success() {
            let err = resp.text().await.unwrap_or_default();
            return Err(AppError::Internal(format!("OpenAI API error: {err}")));
        }

        let json: serde_json::Value = resp.json().await
            .map_err(|e| AppError::Internal(format!("Error parseando respuesta OpenAI: {e}")))?;

        let content = json["choices"][0]["message"]["content"].as_str()
            .ok_or_else(|| AppError::Internal("Respuesta vacia de OpenAI".into()))?;

        self.parse_gpt(content)
    }

    fn parse_gpt(&self, content: &str) -> Result<GptResp> {
        serde_json::from_str(content).or_else(|_| {
            content.find('{').and_then(|start| {
                content.rfind('}').and_then(|end| {
                    serde_json::from_str(&content[start..=end]).ok()
                })
            }).ok_or_else(|| AppError::Internal("No se pudo parsear respuesta GPT".into()))
        })
    }

    fn fallback(&self, device_type: &str) -> GptResp {
        let defaults = [
            ("cpu_percent", "CPU", "%", 75.0, 90.0, "gt", "high"),
            ("memory_percent", "Memoria", "%", 80.0, 95.0, "gt", "high"),
            ("disk_percent", "Disco", "%", 80.0, 95.0, "gt", "medium"),
        ];

        GptResp {
            device_type_detected: Some(device_type.into()),
            thresholds: Some(defaults.iter().map(|(m, n, u, w, c, cmp, p)| GptThreshold {
                metric: m.to_string(),
                display_name: Some(n.to_string()),
                unit: Some(u.to_string()),
                warning: *w,
                critical: *c,
                comparison: Some(cmp.to_string()),
                priority: Some(p.to_string()),
                reason: Some("Umbral por defecto (fallback)".into()),
            }).collect()),
            ignored_metrics: Some(vec![]),
            general_notes: Some("Umbrales generados automaticamente (fallback)".into()),
        }
    }

    async fn persist(&self, device_id: &str, thresholds: &[GptThreshold]) -> Result<usize> {
        sqlx::query("DELETE FROM alert_thresholds WHERE device_id = $1")
            .bind(device_id)
            .execute(&self.pool).await
            .map_err(AppError::Database)?;

        let mut cnt = 0;
        for t in thresholds {
            if sqlx::query(
                r#"INSERT INTO alert_thresholds (device_id, metric_name, warning_threshold, critical_threshold, comparison, duration_seconds)
                   VALUES ($1, $2, $3, $4, $5, $6)
                   ON CONFLICT (device_id, metric_name) DO UPDATE SET
                       warning_threshold = EXCLUDED.warning_threshold,
                       critical_threshold = EXCLUDED.critical_threshold,
                       comparison = EXCLUDED.comparison,
                       duration_seconds = EXCLUDED.duration_seconds,
                       updated_at = NOW()"#)
                .bind(device_id)
                .bind(&t.metric)
                .bind(t.warning)
                .bind(t.critical)
                .bind(t.comparison.as_deref().unwrap_or("gt"))
                .bind(60i32)
                .execute(&self.pool).await.is_ok()
            {
                cnt += 1;
            }
        }
        Ok(cnt)
    }
}
