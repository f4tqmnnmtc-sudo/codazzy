use std::sync::Arc;
use std::time::Duration;
use tokio::{signal, task::JoinSet, time::{interval, timeout}};
use tracing::{debug, error, info, warn};

mod collectors;
mod config;
mod error;
mod metrics;
mod transport;
mod types;

use collectors::{HardwareCollector, NetworkCollector, ProcessCollector, StorageCollector};
use config::Config;
use error::AgentError;
use metrics::SystemMetrics;
use transport::{NatsTransport, TransportConfig};
use types::{Bytes, Celsius, Percent};

const MAX_CONSECUTIVE_FAILS: u32 = 5;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let cfg = Config::load().await?;
    cfg.validate()?;
    
    init_tracing(&cfg)?;

    info!("codazzy-agent v0.2.0");
    info!("nodo: {} ({})", cfg.node.id, cfg.node.environment);
    
    if let Some(ref loc) = cfg.node.location {
        info!("ubicación: {}", loc);
    }

    let hardware = cfg.collection.hardware.enabled
        .then(|| HardwareCollector::new())
        .transpose()?;
    
    let network = cfg.collection.network.enabled
        .then(|| NetworkCollector::new(&cfg))
        .transpose()?;
    
    let storage = cfg.collection.storage.enabled
        .then(|| StorageCollector::new())
        .transpose()?;

    let processes = match &cfg.collection.processes {
        Some(proc_cfg) if proc_cfg.enabled => {
            info!("recolección de procesos cada {}s", proc_cfg.interval);
            Some(ProcessCollector::new())
        }
        _ => {
            debug!("recolección de procesos deshabilitada");
            None
        }
    };

    let transport_cfg = TransportConfig::from_config(&cfg.transport);

    let nats = NatsTransport::new(transport_cfg.clone()).await?;
    info!("NATS: {}", cfg.transport.nats_url);

    let nats_proc = if processes.is_some() {
        Some(NatsTransport::new(transport_cfg).await?)
    } else {
        None
    };

    let cfg = Arc::new(cfg);
    let mut tasks = JoinSet::new();
    
    let cfg_metrics = Arc::clone(&cfg);
    tasks.spawn(async move {
        tick_metrics(hardware, network, storage, nats, cfg_metrics).await
    });

    if let (Some(proc_collector), Some(proc_nats)) = (processes, nats_proc) {
        let cfg_proc = Arc::clone(&cfg);
        tasks.spawn(async move {
            run_process_bucle(proc_collector, proc_nats, cfg_proc).await
        });
    }

    tasks.spawn(async {
        wf_shutdown().await;
        Ok::<_, AgentError>(())
    });

    while let Some(result) = tasks.join_next().await {
        match result {
            Ok(Ok(_)) => {
                info!("tarea finalizada correctamente");
                break;
            }
            Ok(Err(e)) => {
                error!("error en tarea: {}", e);
                break;
            }
            Err(e) => {
                error!("task panic: {}", e);
                break;
            }
        }
    }
    
    tasks.shutdown().await;
    info!("shutdown complete");
    Ok(())
}

fn init_tracing(cfg: &Config) -> Result<(), Box<dyn std::error::Error>> {
    use tracing::Level;
    
    let level = match cfg.logging.level.to_lowercase().as_str() {
        "trace" => Level::TRACE,
        "debug" => Level::DEBUG,
        "warn" | "warning" => Level::WARN,
        "error" | "err" => Level::ERROR,
        _ => Level::INFO,
    };
    
    let sub = tracing_subscriber::fmt()
        .with_max_level(level)
        .with_target(false)
        .with_thread_ids(true);
    
    if cfg.logging.json_format { sub.json().init(); } else { sub.init(); }
    
    Ok(())
}

async fn tick_metrics(
    mut hw: Option<HardwareCollector>,
    mut net: Option<NetworkCollector>,
    mut disk: Option<StorageCollector>,
    mut nats: NatsTransport,
    cfg: Arc<Config>,
) -> Result<(), AgentError> {
    let mut ticker = interval(Duration::from_secs(cfg.collection.interval));
    let mut cons_errors = 0u32;
    let collect_timeout = Duration::from_secs(cfg.collection.interval.saturating_sub(1).max(5));

    loop {
        ticker.tick().await;
        tokio::task::yield_now().await;

        let collect_result = timeout(
            collect_timeout,
            poll(&mut hw, &mut net, &mut disk, &cfg)
        ).await;

        match collect_result {
            Ok(Ok(metrics)) => {
                cons_errors = 0;
                print_data(&metrics, &cfg);
                
                if let Err(e) = nats.send_metrics(metrics).await {
                    warn!("send failed: {}", e);
                }
                
                if nats.should_flush() {
                    if let Err(e) = nats.flush_buffer().await {
                        debug!("error en flush: {}", e);
                    }
                }
                
                let stats = nats.get_stats();
                debug!(
                    "estado NATS: conectado={} buffer={}/{}",
                    stats.connected, stats.buffer_size, stats.buffer_capacity
                );
            }
            Ok(Err(e)) => {
                cons_errors += 1;
                error!("collect error #{}: {}", cons_errors, e);
                
                if cons_errors >= MAX_CONSECUTIVE_FAILS {
                    return Err(e);
                }
                
                let backoff = Duration::from_millis(200 * (1 << cons_errors.min(5)));
                tokio::time::sleep(backoff).await;
            }
            Err(_) => {
                cons_errors += 1;
                error!("collect timeout #{}", cons_errors);
                
                if cons_errors >= MAX_CONSECUTIVE_FAILS {
                    return Err(AgentError::collection_timeout(collect_timeout.as_millis() as u64));
                }
                
                let backoff = Duration::from_millis(500 * (1 << cons_errors.min(4)));
                tokio::time::sleep(backoff).await;
            }
        }
    }
}

async fn poll(
    hw: &mut Option<HardwareCollector>,
    net: &mut Option<NetworkCollector>,
    disk: &mut Option<StorageCollector>,
    cfg: &Config,
) -> Result<SystemMetrics, AgentError> {
    let mut metrics = SystemMetrics::new(&cfg.node.id);

    if let Some(collector) = hw {
        match collector.collect().await {
            Ok(data) => metrics.hardware = data,
            Err(e) => warn!("hw collect: {}", e),
        }
    }
    
    if let Some(collector) = net {
        if let Ok(data) = collector.collect().await {
            metrics.network.interfaces = data.interfaces
                .into_iter()
                .filter(|iface| !filtered(&iface.name, &cfg.metrics.exclude_interfaces))
                .collect();
        }
    }
    
    if let Some(collector) = disk {
        if let Ok(data) = collector.collect().await {
            metrics.storage.disks = data.disks;
            metrics.storage.filesystems = data.filesystems
                .into_iter()
                .filter(|fs| !cfg.metrics.exclude_filesystems.iter().any(|p| fs.mount_point.contains(p)))
                .collect();
        }
    }
    
    Ok(metrics)
}

fn filtered(name: &str, patterns: &[String]) -> bool {
    patterns.iter().any(|p| {
        p.strip_suffix('*').map_or(name == p, |prefix| name.starts_with(prefix))
    })
}

const GB: f64 = 1024.0 * 1024.0 * 1024.0;
const MB: f64 = 1024.0 * 1024.0;

#[inline] fn to_gb(b: Bytes) -> f64 { b as f64 / GB }
#[inline] fn to_mb(b: Bytes) -> f64 { b as f64 / MB }

fn print_data(m: &SystemMetrics, cfg: &Config) {
    let verbose = cfg.metrics.precision == "high";
    
    if !m.hardware.cpu_usage.is_empty() {
        let n = m.hardware.cpu_usage.len();
        let avg: Percent = m.hardware.cpu_usage.iter().sum::<Percent>() / n as Percent;
        info!("cpu: {}c {:.1}%", n, avg);
        
        if verbose {
            for (i, u) in m.hardware.cpu_usage.iter().enumerate() {
                debug!("  core{}: {:.1}%", i, u);
            }
        }
    }

    let mem = &m.hardware.memory_usage;
    if mem.total > 0 {
        let pct = (mem.used as f64 / mem.total as f64) * 100.0;
        info!("mem: {:.1}% ({:.1}/{:.1}G)", pct, to_gb(mem.used), to_gb(mem.total));
    }

    if !m.hardware.thermal_sensors.is_empty() {
        let max = m.hardware.thermal_sensors.iter().map(|s| s.temperature).fold(Celsius::MIN, Celsius::max);
        info!("temp: {}s max {:.0}°C", m.hardware.thermal_sensors.len(), max);
    }

    if !m.network.interfaces.is_empty() {
        let up = m.network.interfaces.iter().filter(|i| i.is_up).count();
        info!("net: {}/{} up", up, m.network.interfaces.len());
        
        for i in &m.network.interfaces {
            debug!("  {}: {} rx:{:.1}M tx:{:.1}M", i.name, if i.is_up {"UP"} else {"DN"}, to_mb(i.bytes_received), to_mb(i.bytes_sent));
        }
    }

    if !m.storage.disks.is_empty() {
        let crit = m.storage.disks.iter().filter(|d| d.utilization > 80.0).count();
        if crit > 0 {
            warn!("disk: {}/{} >80%", crit, m.storage.disks.len());
        } else {
            info!("disk: {}", m.storage.disks.len());
        }
        
        for d in &m.storage.disks {
            let ind = if d.utilization > 90.0 { "!" } else if d.utilization > 80.0 { "*" } else { " " };
            debug!("{}{}: {:.1}%", ind, d.name, d.utilization);
        }
    }

    if !m.storage.filesystems.is_empty() {
        let crit = m.storage.filesystems.iter().filter(|f| f.usage_percent > 90.0).count();
        if crit > 0 {
            warn!("fs: {}/{} >90%", crit, m.storage.filesystems.len());
        }
        
        for f in &m.storage.filesystems {
            let ind = if f.usage_percent > 95.0 { "!" } else if f.usage_percent > 90.0 { "*" } else { " " };
            debug!("{}{}: {:.1}% {:.1}G", ind, f.mount_point, f.usage_percent, to_gb(f.used_space));
        }
    }
}

async fn run_process_bucle(
    mut collector: ProcessCollector,
    mut nats: NatsTransport,
    cfg: Arc<Config>,
) -> Result<(), AgentError> {
    let secs = cfg.collection.processes.as_ref().map_or(300, |p| p.interval);
    let mut ticker = interval(Duration::from_secs(secs));
    info!("proc loop: {}s interval", secs);

    loop {
        ticker.tick().await;
        tokio::task::yield_now().await;

        let summary = collector.collect();

        info!("proc: {} total, {} run, {} svc", summary.total_processes, summary.running_processes, summary.detected_services.len());
        
        for svc in summary.detected_services.iter().take(5) {
            debug!("  {}: {}p {:.1}% {:.0}M", svc.name, svc.process_count, svc.total_cpu, to_mb(svc.total_memory));
        }
        
        let mut metrics = SystemMetrics::new(&cfg.node.id);
        metrics.processes = Some(summary);
        
        if let Err(e) = nats.send_metrics(metrics).await {
            warn!("proc send: {}", e);
        }
    }
}

async fn wf_shutdown() {
    #[cfg(unix)]
    let sigterm = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("no se pudo registrar handler de SIGTERM")
            .recv()
            .await;
    };
    
    #[cfg(not(unix))]
    let sigterm = std::future::pending::<()>();

    tokio::select! {
        _ = signal::ctrl_c() => {
            info!("recibido SIGINT (Ctrl+C)");
        }
        _ = sigterm => {
            info!("recibido SIGTERM");
        }
    }
}


