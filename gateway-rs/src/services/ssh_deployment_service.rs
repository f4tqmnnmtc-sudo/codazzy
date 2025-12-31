use crate::error::{AppError, Result};
use crate::services::cache_service::CacheService;
use crate::services::ssh_service::{SshConfig, SshService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Default)]
#[serde(rename_all = "lowercase")]
pub enum JobStatus { #[default] Pending, Running, Completed, Failed, Cancelled }

impl std::fmt::Display for JobStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", match self { Self::Pending => "pending", Self::Running => "running", Self::Completed => "completed", Self::Failed => "failed", Self::Cancelled => "cancelled" })
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum OsType { Linux, Windows, #[default] Unknown }

impl std::fmt::Display for OsType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result { write!(f, "{}", match self { Self::Linux => "linux", Self::Windows => "windows", Self::Unknown => "unknown" }) }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemoteInstallRequest {
    pub hostname: String, pub port: i32, pub username: String, pub password: Option<String>, pub private_key: Option<String>,
    #[serde(default = "def_os")] pub os_type: String, #[serde(default = "def_nats")] pub nats_url: String,
    pub node_id: Option<String>, pub location: Option<String>, #[serde(default = "def_env")] pub environment: String, #[serde(default)] pub tags: Vec<String>,
}

fn def_os() -> String { "auto".into() }
fn def_nats() -> String { "nats://localhost:4222".into() }
fn def_env() -> String { "production".into() }

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct InstallJob {
    pub job_id: String, pub hostname: String, pub node_id: String, pub status: JobStatus,
    pub progress: i32, pub current_step: String, pub created_at: String, pub started_at: Option<String>,
    pub completed_at: Option<String>, pub error_message: Option<String>, pub logs: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")] pub request: Option<RemoteInstallRequest>,
}

impl InstallJob {
    fn new(jid: String, req: &RemoteInstallRequest) -> Self {
        Self { job_id: jid, hostname: req.hostname.clone(), node_id: req.node_id.clone().unwrap_or_else(|| format!("{}-agent", req.hostname)), status: JobStatus::Pending, progress: 0, current_step: "Esperando inicio...".into(), created_at: chrono::Utc::now().to_rfc3339(), started_at: None, completed_at: None, error_message: None, logs: vec![], request: Some(req.clone()) }
    }
    fn log(&mut self, msg: &str) { self.logs.push(format!("[{}] {}", chrono::Utc::now().to_rfc3339(), msg)); }
}

pub struct SshDeploymentService {
    cache: Arc<CacheService>, ssh: SshService, running: Arc<RwLock<std::collections::HashSet<String>>>,
    prefix: String, max_jobs: usize,
}

impl SshDeploymentService {
    pub fn new(cache: Arc<CacheService>) -> Self { Self { cache, ssh: SshService::new(), running: Arc::new(RwLock::new(std::collections::HashSet::new())), prefix: "deployment_jobs".into(), max_jobs: 5 } }

    pub async fn create_install_job(&self, req: RemoteInstallRequest) -> Result<String> {
        let jid = Uuid::new_v4().to_string(); self.save_job(&InstallJob::new(jid.clone(), &req)).await?; Ok(jid)
    }

    pub async fn exec(&self, jid: String) -> Result<()> {
        { let r = self.running.read().await; if r.len() >= self.max_jobs { return Ok(()) } }
        { self.running.write().await.insert(jid.clone()); }
        let res = self.run_steps(&jid).await;
        { self.running.write().await.remove(&jid); }
        res
    }

    async fn run_steps(&self, jid: &str) -> Result<()> {
        let mut job = self.job(jid).await?.ok_or_else(|| AppError::NotFound(format!("Job {} no encontrado", jid)))?;
        let req = job.request.clone().ok_or_else(|| AppError::Internal("Request del job no encontrado".into()))?;
        job.status = JobStatus::Running; job.started_at = Some(chrono::Utc::now().to_rfc3339()); self.save_job(&job).await?;

        let cfg = SshConfig { hostname: req.hostname.clone(), port: req.port as u16, username: req.username.clone(), password: req.password.clone(), private_key: req.private_key.clone(), timeout_secs: 60 };

        self.upd(&mut job, 10, "Conectando via SSH...").await?;
        if let Err(e) = self.ssh.test_connection(&cfg).await { return self.fail(&mut job, format!("Error de conexion SSH: {e}")).await }

        self.upd(&mut job, 20, "Detectando sistema operativo...").await?;
        let os = self.detect_os(&cfg).await?;

        self.upd(&mut job, 30, "Verificando dependencias...").await?;
        if let Err(e) = self.verify_deps(&cfg, &os).await { return self.fail(&mut job, format!("Dependencias faltantes: {e}")).await }

        self.upd(&mut job, 40, "Generando configuracion del agente...").await?;
        let config_content = self.gen_config(&req, &os);

        self.upd(&mut job, 50, "Transfiriendo paquete del agente...").await?;
        if let Err(e) = self.transfer_pkg(&cfg, &os, &req.username).await { return self.fail(&mut job, format!("Error transfiriendo paquete: {e}")).await }

        self.upd(&mut job, 70, "Instalando agente...").await?;
        if let Err(e) = self.install(&cfg, &os, &config_content, &req).await { return self.fail(&mut job, format!("Error instalando agente: {e}")).await }

        self.upd(&mut job, 85, "Configurando servicio...").await?;
        let _ = self.config_svc(&cfg, &os, &req.password).await;

        self.upd(&mut job, 95, "Verificando instalacion...").await?;
        if let Err(e) = self.verify(&cfg, &os, &req.password).await { return self.fail(&mut job, format!("Verificacion fallida: {e}")).await }

        job.status = JobStatus::Completed; job.progress = 100; job.current_step = "Instalacion completada exitosamente".into();
        job.completed_at = Some(chrono::Utc::now().to_rfc3339()); job.log("Agente instalado y funcionando correctamente");
        self.save_job(&job).await
    }

    async fn fail(&self, job: &mut InstallJob, err: String) -> Result<()> {
        job.status = JobStatus::Failed; job.error_message = Some(err.clone()); job.completed_at = Some(chrono::Utc::now().to_rfc3339());
        self.save_job(job).await?; Err(AppError::Internal(err))
    }

    async fn detect_os(&self, cfg: &SshConfig) -> Result<OsType> {
        let r = self.ssh.execute_command(cfg, "uname -s").await?;
        if r.stdout.to_lowercase().contains("linux") || r.stdout.to_lowercase().contains("darwin") { return Ok(OsType::Linux) }
        if let Ok(r) = self.ssh.execute_command(cfg, "ver").await { if r.stdout.to_lowercase().contains("windows") { return Ok(OsType::Windows) } }
        Ok(OsType::Linux)
    }

    async fn verify_deps(&self, cfg: &SshConfig, os: &OsType) -> Result<()> {
        match os {
            OsType::Linux => { let r = self.ssh.execute_command(cfg, "which curl || which wget").await?; if r.exit_code != 0 { return Err(AppError::Validation("curl o wget no encontrado".into())) } Ok(()) }
            OsType::Windows => { let r = self.ssh.execute_command(cfg, "powershell -Command \"Get-Command curl\"").await?; if r.exit_code != 0 { return Err(AppError::Validation("PowerShell o curl no disponible".into())) } Ok(()) }
            OsType::Unknown => Ok(()),
        }
    }

    fn gen_config(&self, req: &RemoteInstallRequest, os: &OsType) -> String {
        let nid = req.node_id.clone().unwrap_or_else(|| format!("{}-agent", req.hostname));
        let loc = req.location.clone().unwrap_or_else(|| "remote".into());
        let ts = !matches!(os, OsType::Windows);
        format!(r#"[node]
id = "{nid}"
environment = "{env}"
location = "{loc}"
tags = {tags:?}

[collection]
interval_seconds = 5

[collection.hardware]
enabled = true
cpu_detailed = true
memory_detailed = true
temperature_sensors = {ts}
gpu_metrics = false
power_metrics = false

[collection.network]
enabled = true
interface_details = true
bandwidth_monitoring = true
latency_monitoring = false
dns_metrics = false

[collection.storage]
enabled = true
disk_io_detailed = true
filesystem_monitoring = true
smart_metrics = {ts}

[collection.processes]
enabled = true
interval_seconds = 10

[transport]
nats_url = "{nats}"
topic_prefix = "metrics"
buffer_size = 1000
compression = true
retry_attempts = 3
batch_size = 10
flush_interval_seconds = 30
connection_timeout_seconds = 10

[metrics]
precision = "medium"
retention_hours = 24
exclude_interfaces = ["lo", "docker0", "veth*"]
exclude_filesystems = ["tmpfs", "devtmpfs", "sysfs", "proc"]

[metrics.custom_labels]
environment = "{env}"
location = "{loc}"

[logging]
level = "info"
max_file_size_mb = 10
max_files = 5
json_format = false
"#, nid = nid, env = req.environment, loc = loc, tags = req.tags, ts = ts, nats = req.nats_url)
    }

    async fn transfer_pkg(&self, cfg: &SshConfig, os: &OsType, user: &str) -> Result<()> {
        match os {
            OsType::Linux => {
                let paths = ["/opt/agent-packages/codazzy-agent-0.2.0-linux-x86_64.tar.gz", "./agent/build-portable/codazzy-agent-0.2.0-linux-x86_64.tar.gz", "../agent/build-portable/codazzy-agent-0.2.0-linux-x86_64.tar.gz"];
                let pkg = paths.iter().find(|p| std::path::Path::new(p).exists()).ok_or_else(|| AppError::NotFound("Paquete portable del agente no encontrado".into()))?;
                let data = tokio::fs::read(pkg).await.map_err(|e| AppError::Internal(format!("Error leyendo paquete: {e}")))?;
                let (rp, ed) = (format!("/home/{user}/codazzy-agent-portable.tar.gz"), format!("/home/{user}/codazzy-install"));
                self.ssh.upload_file(cfg, data, &rp).await?;
                let r = self.ssh.execute_command(cfg, &format!("rm -rf {ed} && mkdir -p {ed} && cd {ed} && tar -xzf {rp}")).await?;
                if r.exit_code != 0 { return Err(AppError::Internal(format!("Error extrayendo paquete: {}", r.stderr))) }
                Ok(())
            }
            OsType::Windows => Err(AppError::Validation("Instalacion en Windows no implementada".into())),
            OsType::Unknown => Err(AppError::Validation("Sistema operativo no soportado".into())),
        }
    }

    async fn install(&self, cfg: &SshConfig, os: &OsType, config: &str, req: &RemoteInstallRequest) -> Result<()> {
        match os {
            OsType::Linux => {
                let pd = format!("/home/{}/codazzy-install/codazzy-agent-0.2.0-linux-x86_64", req.username);
                let cp = format!("{pd}/config.toml");
                let r = self.ssh.execute_command(cfg, &format!("cat > {cp} << 'EOFCONFIG'\n{config}\nEOFCONFIG")).await?;
                if r.exit_code != 0 { return Err(AppError::Internal(format!("Error creando config: {}", r.stderr))) }
                let nh = req.nats_url.replace("nats://", "").split(':').next().unwrap_or("localhost").to_string();
                let cmd = req.password.as_ref().map(|p| format!("cd {pd} && chmod +x install.sh && export NATS_HOST=\"{nh}\" && echo '{p}' | sudo -S -E ./install.sh")).unwrap_or_else(|| format!("cd {pd} && chmod +x install.sh && export NATS_HOST=\"{nh}\" && sudo -E ./install.sh"));
                let r = self.ssh.execute_command(cfg, &cmd).await?;
                if r.exit_code != 0 { return Err(AppError::Internal(format!("Error en instalacion: {}\nOutput: {}", r.stderr, r.stdout))) }
                Ok(())
            }
            _ => Err(AppError::Validation("Sistema operativo no soportado".into())),
        }
    }

    async fn config_svc(&self, cfg: &SshConfig, os: &OsType, pwd: &Option<String>) -> Result<()> {
        if !matches!(os, OsType::Linux) { return Ok(()) }
        let has_systemd = self.ssh.execute_command(cfg, "which systemctl").await?.exit_code == 0;
        let cmds: Vec<String> = if has_systemd {
            pwd.as_ref().map(|p| vec![format!("echo '{p}' | sudo -S systemctl daemon-reload"), format!("echo '{p}' | sudo -S systemctl enable codazzy-agent"), format!("echo '{p}' | sudo -S systemctl start codazzy-agent")]).unwrap_or_else(|| vec!["sudo systemctl daemon-reload".into(), "sudo systemctl enable codazzy-agent".into(), "sudo systemctl start codazzy-agent".into()])
        } else {
            pwd.as_ref().map(|p| vec![format!("echo '{p}' | sudo -S rc-update add codazzy-agent default"), format!("echo '{p}' | sudo -S rc-service codazzy-agent start")]).unwrap_or_else(|| vec!["sudo rc-update add codazzy-agent default".into(), "sudo rc-service codazzy-agent start".into()])
        };
        for c in cmds { let _ = self.ssh.execute_command(cfg, &c).await; }
        Ok(())
    }

    async fn verify(&self, cfg: &SshConfig, os: &OsType, pwd: &Option<String>) -> Result<()> {
        if !matches!(os, OsType::Linux) { return Ok(()) }
        if self.ssh.execute_command(cfg, "test -f /opt/codazzy-agent/codazzy-agent").await?.exit_code != 0 { return Err(AppError::Internal("Binario del agente no encontrado en /opt/codazzy-agent/".into())) }
        let cmd = pwd.as_ref().map(|p| format!("echo '{p}' | sudo -S systemctl is-active codazzy-agent 2>/dev/null || pgrep -f codazzy-agent")).unwrap_or_else(|| "systemctl is-active codazzy-agent 2>/dev/null || pgrep -f codazzy-agent".into());
        let _ = self.ssh.execute_command(cfg, &cmd).await?;
        Ok(())
    }

    async fn upd(&self, job: &mut InstallJob, pct: i32, step: &str) -> Result<()> { job.progress = pct; job.current_step = step.into(); job.log(step); self.save_job(job).await }

    async fn save_job(&self, job: &InstallJob) -> Result<()> {
        let val = serde_json::to_string(job).map_err(|e| AppError::Internal(format!("Error serializando job: {e}")))?;
        self.cache.set_raw(&format!("{}:{}", self.prefix, job.job_id), &val, 3600).await
    }

    pub async fn job(&self, jid: &str) -> Result<Option<InstallJob>> {
        self.cache.get_raw(&format!("{}:{}", self.prefix, jid)).await?.map(|v| serde_json::from_str(&v).map_err(|e| AppError::Internal(format!("Error deserializando job: {e}")))).transpose()
    }

    pub async fn job_status(&self, jid: &str) -> Result<Option<serde_json::Value>> {
        Ok(self.job(jid).await?.map(|j| serde_json::json!({ "job_id": j.job_id, "hostname": j.hostname, "status": j.status.to_string(), "progress": j.progress, "current_step": j.current_step, "created_at": j.created_at, "started_at": j.started_at, "completed_at": j.completed_at, "error_message": j.error_message, "logs": j.logs.iter().rev().take(10).collect::<Vec<_>>() })))
    }

    pub async fn list_jobs(&self, limit: usize) -> Result<Vec<serde_json::Value>> {
        let keys = self.cache.keys(&format!("{}:*", self.prefix)).await?;
        let mut jobs = Vec::new();
        for k in keys.into_iter().take(limit) { if let Some(st) = self.job_status(k.split(':').last().unwrap_or("")).await? { jobs.push(st); } }
        jobs.sort_by(|a, b| b["created_at"].as_str().unwrap_or("").cmp(a["created_at"].as_str().unwrap_or("")));
        Ok(jobs)
    }

    pub async fn cancel_job(&self, jid: &str) -> Result<bool> {
        let Some(mut job) = self.job(jid).await? else { return Ok(false) };
        if matches!(job.status, JobStatus::Completed | JobStatus::Failed | JobStatus::Cancelled) { return Ok(false) }
        job.status = JobStatus::Cancelled; job.completed_at = Some(chrono::Utc::now().to_rfc3339()); job.error_message = Some("Job cancelado por el usuario".into());
        self.save_job(&job).await?; self.running.write().await.remove(jid); Ok(true)
    }

    pub async fn clear_all_jobs(&self) -> Result<usize> {
        let keys = self.cache.keys(&format!("{}:*", self.prefix)).await?; let cnt = keys.len();
        for k in keys { self.cache.delete(&k).await?; }
        self.running.write().await.clear(); Ok(cnt)
    }
}
