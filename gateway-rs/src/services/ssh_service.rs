use crate::error::{AppError, Result};
use ssh2::Session;
use std::io::Read;
use std::net::TcpStream;
use std::time::Duration;

#[derive(Debug, Clone, Default)]
pub struct SshConfig {
    pub hostname: String,
    pub port: u16,
    pub username: String,
    pub password: Option<String>,
    pub private_key: Option<String>,
    pub timeout_secs: u64,
}

#[derive(Debug)]
pub struct CommandResult {
    pub stdout: String,
    pub stderr: String,
    pub exit_code: i32,
}

pub struct SshService;

impl SshService {
    pub fn new() -> Self { Self }

    fn connect_sync(cfg: &SshConfig) -> Result<Session> {
        let addr = format!("{}:{}", cfg.hostname, cfg.port);
        let tcp = TcpStream::connect(&addr)
            .map_err(|e| AppError::Ssh(format!("connect {addr}: {e}")))?;

        let timeout = Duration::from_secs(cfg.timeout_secs);
        tcp.set_read_timeout(Some(timeout)).ok();
        tcp.set_write_timeout(Some(timeout)).ok();

        let mut sess = Session::new()
            .map_err(|e| AppError::Ssh(format!("session: {e}")))?;
        sess.set_tcp_stream(tcp);
        sess.handshake()
            .map_err(|e| AppError::Ssh(format!("handshake: {e}")))?;

        match (&cfg.password, &cfg.private_key) {
            (Some(pw), _) => sess.userauth_password(&cfg.username, pw)
                .map_err(|e| AppError::Ssh(format!("auth pw: {e}")))?,
            (_, Some(key)) => sess.userauth_pubkey_memory(&cfg.username, None, key, None)
                .map_err(|e| AppError::Ssh(format!("auth key: {e}")))?,
            _ => return Err(AppError::Validation("password o clave requerida".into())),
        }

        if !sess.authenticated() {
            return Err(AppError::Ssh("auth fallida".into()));
        }
        Ok(sess)
    }

    fn exec_sync(sess: &Session, cmd: &str) -> Result<CommandResult> {
        let mut ch = sess.channel_session()
            .map_err(|e| AppError::Ssh(format!("channel: {e}")))?;
        ch.exec(cmd)
            .map_err(|e| AppError::Ssh(format!("exec: {e}")))?;

        let mut stdout = String::new();
        ch.read_to_string(&mut stdout)
            .map_err(|e| AppError::Ssh(format!("stdout: {e}")))?;

        let mut stderr = String::new();
        ch.stderr().read_to_string(&mut stderr)
            .map_err(|e| AppError::Ssh(format!("stderr: {e}")))?;

        ch.wait_close().ok();
        let code = ch.exit_status().unwrap_or(-1);

        Ok(CommandResult { stdout, stderr, exit_code: code })
    }

    pub async fn execute_command(&self, cfg: &SshConfig, cmd: &str) -> Result<CommandResult> {
        let cfg = cfg.clone();
        let cmd = cmd.to_string();
        tokio::task::spawn_blocking(move || {
            let sess = Self::connect_sync(&cfg)?;
            Self::exec_sync(&sess, &cmd)
        })
        .await
        .map_err(|e| AppError::Ssh(format!("task: {e}")))?
    }

    pub async fn read_file(&self, cfg: &SshConfig, path: &str) -> Result<String> {
        let r = self.execute_command(cfg, &format!("cat {path}")).await?;
        if r.exit_code != 0 {
            return Err(AppError::Ssh(format!("read {path}: {}", r.stderr)));
        }
        Ok(r.stdout)
    }

    pub async fn write_file(&self, cfg: &SshConfig, path: &str, content: &str) -> Result<()> {
        let _ = self.execute_command(cfg, &format!("cp {path} {path}.bak 2>/dev/null || true")).await;
        let esc = content.replace('\'', "'\\''");
        let r = self.execute_command(cfg, &format!("cat > {path} << 'EOFCFG'\n{esc}\nEOFCFG")).await?;
        if r.exit_code != 0 {
            return Err(AppError::Ssh(format!("write {path}: {}", r.stderr)));
        }
        Ok(())
    }

    pub async fn file_exists(&self, cfg: &SshConfig, path: &str) -> Result<bool> {
        let r = self.execute_command(cfg, &format!("test -f {path} && echo 1 || echo 0")).await?;
        Ok(r.stdout.trim() == "1")
    }

    pub async fn test_connection(&self, cfg: &SshConfig) -> Result<bool> {
        let r = self.execute_command(cfg, "echo ok").await?;
        Ok(r.stdout.contains("ok"))
    }

    pub async fn upload_file(&self, cfg: &SshConfig, data: Vec<u8>, remote: &str) -> Result<()> {
        let cfg = cfg.clone();
        let remote = remote.to_string();
        tokio::task::spawn_blocking(move || {
            use std::io::Write;
            let sess = Self::connect_sync(&cfg)?;
            let sftp = sess.sftp()
                .map_err(|e| AppError::Ssh(format!("sftp: {e}")))?;
            let mut f = sftp.create(std::path::Path::new(&remote))
                .map_err(|e| AppError::Ssh(format!("create {remote}: {e}")))?;
            f.write_all(&data)
                .map_err(|e| AppError::Ssh(format!("write: {e}")))?;
            Ok(())
        })
        .await
        .map_err(|e| AppError::Ssh(format!("task: {e}")))?
    }
}

impl Default for SshService {
    fn default() -> Self { Self::new() }
}
