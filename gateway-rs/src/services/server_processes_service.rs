use crate::error::{AppError, Result};
use crate::models::metrics::{ProcessMetrics, ProcessSummary, ServiceInfo};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::{postgres::PgPool, FromRow};

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoredProcess {
    pub id: i32,
    pub device_id: String,
    pub process_name: String,
    pub pid: Option<i32>,
    pub cpu_usage: Option<f32>,
    pub memory_bytes: Option<i64>,
    pub memory_percent: Option<f32>,
    pub status: Option<String>,
    pub exe_path: Option<String>,
    pub command: Option<String>,
    pub collected_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct StoredService {
    pub id: i32,
    pub device_id: String,
    pub service_name: String,
    pub display_name: Option<String>,
    pub status: Option<String>,
    pub process_count: Option<i32>,
    pub total_cpu: Option<f32>,
    pub total_memory: Option<i64>,
    pub collected_at: DateTime<Utc>,
}

pub struct ServerProcessesService {
    pool: PgPool,
}

impl ServerProcessesService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn save_processes(&self, device_id: &str, summary: &ProcessSummary) -> Result<usize> {
        let now = Utc::now();
        let mut saved = 0;

        let _ = sqlx::query("DELETE FROM server_processes WHERE device_id = $1 AND collected_at < NOW() - INTERVAL '24 hours'")
            .bind(device_id).execute(&self.pool).await;

        for proc in &summary.top_cpu_processes {
            if self.save_process(device_id, proc, &now).await.is_ok() {
                saved += 1;
            }
        }

        for proc in &summary.top_memory_processes {
            if !summary.top_cpu_processes.iter().any(|p| p.pid == proc.pid) {
                if self.save_process(device_id, proc, &now).await.is_ok() {
                    saved += 1;
                }
            }
        }

        for svc in &summary.detected_services {
            let _ = self.save_service(device_id, svc, &now).await;
        }

        Ok(saved)
    }

    async fn save_process(
        &self,
        device_id: &str,
        p: &ProcessMetrics,
        ts: &DateTime<Utc>,
    ) -> Result<()> {
        let cmd = (!p.cmd.is_empty()).then(|| p.cmd.join(" "));
        sqlx::query(
            r#"INSERT INTO server_processes (device_id, process_name, pid, cpu_usage, memory_bytes, memory_percent, status, exe_path, command, collected_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (device_id, pid, collected_at) DO UPDATE SET process_name = EXCLUDED.process_name, cpu_usage = EXCLUDED.cpu_usage, memory_bytes = EXCLUDED.memory_bytes, memory_percent = EXCLUDED.memory_percent, status = EXCLUDED.status, exe_path = EXCLUDED.exe_path, command = EXCLUDED.command"#
        )
        .bind(device_id).bind(&p.name).bind(p.pid as i32).bind(p.cpu_usage as f32).bind(p.memory_bytes as i64).bind(p.memory_percent as f32).bind(&p.status).bind(&p.exe_path).bind(&cmd).bind(ts)
        .execute(&self.pool).await.map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    async fn save_service(
        &self,
        device_id: &str,
        s: &ServiceInfo,
        ts: &DateTime<Utc>,
    ) -> Result<()> {
        sqlx::query(
            r#"INSERT INTO server_services (device_id, service_name, display_name, status, process_count, total_cpu, total_memory, collected_at)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8)
            ON CONFLICT (device_id, service_name, collected_at) DO UPDATE SET display_name = EXCLUDED.display_name, status = EXCLUDED.status, process_count = EXCLUDED.process_count, total_cpu = EXCLUDED.total_cpu, total_memory = EXCLUDED.total_memory"#
        )
        .bind(device_id).bind(&s.name).bind(&s.display_name).bind(&s.status).bind(s.process_count as i32).bind(s.total_cpu as f32).bind(s.total_memory as i64).bind(ts)
        .execute(&self.pool).await.map_err(|e| AppError::Database(e))?;
        Ok(())
    }

    pub async fn latest_processes(
        &self,
        device_id: &str,
        limit: i32,
    ) -> Result<Vec<StoredProcess>> {
        sqlx::query_as(
            r#"SELECT id, device_id, process_name, pid, cpu_usage, memory_bytes, memory_percent, status, exe_path, command, collected_at
            FROM server_processes WHERE device_id = $1 ORDER BY collected_at DESC, cpu_usage DESC NULLS LAST LIMIT $2"#
        ).bind(device_id).bind(limit as i64).fetch_all(&self.pool).await.map_err(|e| AppError::Database(e))
    }

    pub async fn latest_services(&self, device_id: &str) -> Result<Vec<StoredService>> {
        sqlx::query_as(
            r#"SELECT id, device_id, service_name, display_name, status, process_count, total_cpu, total_memory, collected_at
            FROM server_services WHERE device_id = $1 ORDER BY collected_at DESC, total_cpu DESC NULLS LAST"#
        ).bind(device_id).fetch_all(&self.pool).await.map_err(|e| AppError::Database(e))
    }

    pub async fn processes_for_ai(
        &self,
        device_id: &str,
    ) -> Result<Vec<StoredProcess>> {
        sqlx::query_as(
            r#"SELECT DISTINCT ON (process_name) id, device_id, process_name, pid, cpu_usage, memory_bytes, memory_percent, status, exe_path, command, collected_at
            FROM server_processes WHERE device_id = $1 AND collected_at > NOW() - INTERVAL '1 hour'
            ORDER BY process_name, collected_at DESC, cpu_usage DESC NULLS LAST LIMIT 20"#
        ).bind(device_id).fetch_all(&self.pool).await.map_err(|e| AppError::Database(e))
    }
}
