use crate::api::routes::AppState;
use crate::error::AppError;
use crate::models::server_metadata::CreateAgentConnectionRequest;
use crate::services::ssh_deployment_service::RemoteInstallRequest;
use crate::services::ssh_service::{SshConfig, SshService};
use axum::{extract::{Path, Query, State}, Json};
use serde::Deserialize;
use tracing::{error, info};

#[derive(Debug, Deserialize)]
pub struct RemoteInstallQuery {
    #[serde(default = "def_limit")]
    pub limit: i32,
}

fn def_limit() -> i32 { 50 }
fn def_port() -> i32 { 22 }
fn def_restart() -> bool { true }

#[derive(Debug, Deserialize)]
pub struct RemoteConfigRequest {
    pub hostname: String,
    #[serde(default = "def_port")]
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub config_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ConfigUpdateRequest {
    pub hostname: String,
    #[serde(default = "def_port")]
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub config_content: String,
    #[serde(default = "def_restart")]
    pub restart_agent: bool,
    pub config_path: Option<String>,
    pub agent_path: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct SshHealthCheckQuery {
    pub username: String,
    #[serde(default = "def_port")]
    pub port: i32,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UninstallRequest {
    pub node_id: String,
    pub hostname: String,
    #[serde(default = "def_port")]
    pub port: i32,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
}

fn require_auth(pass: &Option<String>, key: &Option<String>) -> Result<(), AppError> {
    if pass.is_none() && key.is_none() {
        return Err(AppError::Validation("password o private_key requerido".into()));
    }
    Ok(())
}

fn mk_ssh_cfg(host: &str, port: i32, user: &str, pass: &Option<String>, key: &Option<String>, timeout: u64) -> SshConfig {
    SshConfig {
        hostname: host.into(),
        port: port as u16,
        username: user.into(),
        password: pass.clone(),
        private_key: key.clone(),
        timeout_secs: timeout,
    }
}

fn sudo_wrap(cmd: &str, pass: &str) -> String {
    let esc = pass.replace('\'', "'\\''");
    if pass.is_empty() {
        format!("{cmd} 2>/dev/null || sudo {cmd} 2>/dev/null || true")
    } else {
        format!("{cmd} 2>/dev/null || echo '{esc}' | sudo -S {cmd} 2>/dev/null || true")
    }
}

const CFG_PATHS: &[&str] = &[
    "/opt/codazzy-agent/config/config.toml",
    "/etc/codazzy/agent/config.toml",
    "/opt/codazzy-agent/config.toml",
];

pub async fn list_jobs(
    State(st): State<AppState>,
    Query(q): Query<RemoteInstallQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let jobs = st.ssh_deployment_service.list_jobs(q.limit as usize).await?;
    Ok(Json(jobs))
}

pub async fn create_job(
    State(st): State<AppState>,
    Json(req): Json<RemoteInstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_auth(&req.password, &req.private_key)?;
    let host = req.hostname.clone();
    info!("install job: {host}");

    let job_id = st.ssh_deployment_service.create_install_job(req.clone()).await?;
    let (deploy_svc, conn_svc) = (st.ssh_deployment_service.clone(), st.agent_connection_service.clone());
    let jid = job_id.clone();

    tokio::spawn(async move {
        if deploy_svc.exec(jid.clone()).await.is_ok() {
            let nid = req.node_id.clone().unwrap_or_else(|| format!("{}-agent", req.hostname));
            let conn = CreateAgentConnectionRequest {
                node_id: nid.clone(),
                ssh_hostname: req.hostname.clone(),
                ssh_port: req.port,
                ssh_username: Some(req.username.clone()),
                config_path: Some("/opt/codazzy-agent/config/config.toml".into()),
                agent_path: Some("/opt/codazzy-agent/codazzy-agent".into()),
                location: req.location.clone(),
                environment: req.environment.clone(),
                tags: if req.tags.is_empty() { None } else { Some(req.tags.clone()) },
                os_type: Some(req.os_type.clone()),
                installation_method: "remote_ssh".into(),
                job_id: Some(jid),
                notes: None,
            };
            if let Err(e) = conn_svc.create_connection(conn).await {
                error!("save conn {nid}: {e}");
            }
        }
    });

    Ok(Json(serde_json::json!({
        "job_id": job_id, "status": "pending",
        "message": format!("Job creado para {host}")
    })))
}

pub async fn show_job(
    State(st): State<AppState>,
    Path(jid): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    st.ssh_deployment_service.job_status(&jid).await?
        .map(Json)
        .ok_or_else(|| AppError::NotFound(format!("job {jid} not found")))
}

pub async fn cancel_job(
    State(st): State<AppState>,
    Path(jid): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    if st.ssh_deployment_service.cancel_job(&jid).await? {
        Ok(Json(serde_json::json!({"message": format!("Job {jid} cancelado")})))
    } else {
        Err(AppError::Validation(format!("Job {jid} no cancelable")))
    }
}

pub async fn clear_jobs(
    State(st): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let n = st.ssh_deployment_service.clear_all_jobs().await?;
    Ok(Json(serde_json::json!({"message": format!("{n} jobs eliminados"), "deleted_count": n})))
}

pub async fn installed_servers(State(st): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let nodes = st.influx_service.active_nodes("5m").await?;
    let conns = st.agent_connection_service.list_connections().await?;
    let cmap: std::collections::HashMap<_, _> = conns.iter().map(|c| (c.node_id.clone(), c)).collect();

    let servers: Vec<_> = nodes.iter().map(|nid| {
        let c = cmap.get(nid);
        serde_json::json!({
            "node_id": nid,
            "hostname": c.map(|x| x.ssh_hostname.clone()).unwrap_or_else(|| nid.clone()),
            "status": "online",
            "os_type": c.and_then(|x| x.os_type.clone()).unwrap_or_else(|| "linux".into()),
            "config_path": c.and_then(|x| x.config_path.clone()).unwrap_or_else(|| "/opt/codazzy-agent/config/config.toml".into()),
            "agent_path": c.and_then(|x| x.agent_path.clone()).unwrap_or_else(|| "/opt/codazzy-agent/codazzy-agent".into()),
            "ssh_port": c.map(|x| x.ssh_port).unwrap_or(22),
            "ssh_username": c.and_then(|x| x.ssh_username.clone()),
            "location": c.and_then(|x| x.location.clone()),
            "environment": c.map(|x| x.environment.clone()).unwrap_or_else(|| "production".into()),
            "has_connection": c.is_some(),
        })
    }).collect();

    Ok(Json(serde_json::json!({"servers": servers, "total": servers.len()})))
}

pub async fn read_config(Json(req): Json<RemoteConfigRequest>) -> Result<Json<serde_json::Value>, AppError> {
    require_auth(&req.password, &req.private_key)?;

    let cfg = mk_ssh_cfg(&req.hostname, req.port, &req.username, &req.password, &req.private_key, 30);
    let ssh = SshService::new();

    let paths: Vec<String> = req.config_path.as_ref()
        .map(|p| vec![p.clone()])
        .unwrap_or_else(|| CFG_PATHS.iter().map(|s| (*s).into()).collect());

    for path in &paths {
        let r = ssh.execute_command(&cfg, &format!("test -f {path} && cat {path}")).await?;
        if r.exit_code == 0 && !r.stdout.trim().is_empty() {
            return Ok(Json(serde_json::json!({
                "success": true, "hostname": req.hostname,
                "config_path": path, "config": r.stdout
            })));
        }
    }

    Ok(Json(serde_json::json!({
        "success": false,
        "error": format!("config no encontrado: {}", paths.join(", "))
    })))
}

pub async fn write_config(Json(req): Json<ConfigUpdateRequest>) -> Result<Json<serde_json::Value>, AppError> {
    require_auth(&req.password, &req.private_key)?;

    let cfg = mk_ssh_cfg(&req.hostname, req.port, &req.username, &req.password, &req.private_key, 30);
    let ssh = SshService::new();
    let pass = req.password.clone().unwrap_or_default();
    let esc_pass = pass.replace('\'', "'\\''");
    let esc_content = req.config_content.replace('\'', "'\\''");

    let tmp = format!("/tmp/codazzy_cfg_{}.toml", std::process::id());
    let r = ssh.execute_command(&cfg, &format!("cat > {tmp} << 'EOF'\n{esc_content}\nEOF")).await?;
    if r.exit_code != 0 {
        return Err(AppError::Ssh(format!("tmp write: {}", r.stderr.trim())));
    }

    let targets = ["/etc/codazzy/agent/config.toml", "/opt/codazzy-agent/config/config.toml"];
    let (mut ok, mut errs) = (Vec::new(), Vec::new());

    for path in targets {
        if let Some(dir) = std::path::Path::new(path).parent() {
            let _ = ssh.execute_command(&cfg, &sudo_wrap(&format!("mkdir -p {}", dir.display()), &pass)).await;
        }

        let cp = ssh.execute_command(&cfg, &format!("cp {tmp} {path}")).await?;
        if cp.exit_code == 0 {
            ok.push(path);
        } else {
            let sudo_cp = if pass.is_empty() {
                format!("sudo cp {tmp} {path}")
            } else {
                format!("echo '{esc_pass}' | sudo -S cp {tmp} {path}")
            };
            let r = ssh.execute_command(&cfg, &sudo_cp).await?;
            if r.exit_code == 0 { ok.push(path); }
            else { errs.push(format!("{path}: {}", r.stderr.trim())); }
        }
    }

    let _ = ssh.execute_command(&cfg, &format!("rm -f {tmp}")).await;

    if ok.is_empty() {
        return Err(AppError::Ssh(format!("ninguno actualizado: {}", errs.join("; "))));
    }

    let restart = if req.restart_agent {
        let _ = ssh.execute_command(&cfg, &sudo_wrap("systemctl restart codazzy-agent", &pass)).await;
        let st = ssh.execute_command(&cfg, "systemctl is-active codazzy-agent 2>/dev/null || echo 'unknown'").await?;
        Some(serde_json::json!({ "status": st.stdout.trim() }))
    } else { None };

    Ok(Json(serde_json::json!({ "success": true, "hostname": req.hostname, "updated": ok, "errors": errs, "restart": restart })))
}

pub async fn ssh_health(
    Path(hostname): Path<String>,
    Query(q): Query<SshHealthCheckQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    if q.password.is_none() && q.private_key.is_none() {
        return Ok(Json(serde_json::json!({"status": "failed", "error": "auth requerida"})));
    }

    let cfg = mk_ssh_cfg(&hostname, q.port, &q.username, &q.password, &q.private_key, 15);
    let ssh = SshService::new();

    match ssh.test_connection(&cfg).await {
        Ok(true) => {
            let os = ssh.execute_command(&cfg, "uname -s 2>/dev/null || echo Windows").await
                .map(|r| {
                    let s = r.stdout.trim().to_lowercase();
                    if s.contains("linux") { "linux" }
                    else if s.contains("darwin") { "macos" }
                    else { "linux" }
                })
                .unwrap_or("linux");
            Ok(Json(serde_json::json!({ "status": "ok", "hostname": hostname, "os": os })))
        }
        Ok(false) => Ok(Json(serde_json::json!({"status": "failed", "error": "auth rechazada"}))),
        Err(e) => Ok(Json(serde_json::json!({"status": "failed", "error": e.to_string()}))),
    }
}

pub async fn uninstall(
    State(st): State<AppState>,
    Json(req): Json<UninstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    require_auth(&req.password, &req.private_key)?;
    info!("uninstall {} @ {}", req.node_id, req.hostname);

    let cfg = mk_ssh_cfg(&req.hostname, req.port, &req.username, &req.password, &req.private_key, 60);
    let ssh = SshService::new();

    ssh.test_connection(&cfg).await.map_err(|e| AppError::Ssh(format!("conn: {e}")))?;

    let pass = req.password.clone().unwrap_or_default();
    let (mut steps, mut errs): (Vec<&str>, Vec<String>) = (vec![], vec![]);

    for cmd in ["systemctl stop codazzy-agent", "systemctl disable codazzy-agent"] {
        let _ = ssh.execute_command(&cfg, &sudo_wrap(cmd, &pass)).await;
    }
    steps.push("servicio detenido");

    let _ = ssh.execute_command(&cfg, &sudo_wrap("rm -f /etc/systemd/system/codazzy-agent.service", &pass)).await;
    let _ = ssh.execute_command(&cfg, &sudo_wrap("systemctl daemon-reload", &pass)).await;
    steps.push("unit eliminado");

    let user_dir = format!("/home/{}/codazzy-install", req.username);
    for dir in ["/opt/codazzy-agent", "/etc/codazzy/agent", &user_dir] {
        let _ = ssh.execute_command(&cfg, &sudo_wrap(&format!("rm -rf {dir}"), &pass)).await;
    }
    steps.push("dirs eliminados");

    let _ = st.agent_connection_service.delete_connection(&req.node_id).await;
    steps.push("registro BD");

    if let Ok(r) = ssh.execute_command(&cfg, "pgrep -f codazzy-agent || echo stopped").await {
        if r.stdout.trim() != "stopped" && r.stdout.trim().parse::<i32>().is_ok() {
            errs.push("proceso aun corriendo".into());
        }
    }

    let _ = st.cache_service.delete_pattern("active_nodes_*").await;
    let _ = st.cache_service.delete_pattern("overview_*").await;

    Ok(Json(serde_json::json!({
        "success": errs.is_empty(),
        "node_id": req.node_id, "hostname": req.hostname,
        "steps": steps, "errors": errs
    })))
}
