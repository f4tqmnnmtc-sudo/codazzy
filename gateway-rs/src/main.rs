use std::{net::SocketAddr, sync::Arc};
use tokio::signal;
use tracing::{error, info, warn};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use codazzy_gateway::api::{create_router, routes::AppState};
use codazzy_gateway::config::CONFIG;
use codazzy_gateway::consumer::JetStreamConsumer;
use codazzy_gateway::processing::InfluxWriter;
use codazzy_gateway::services::{
    AgentConnectionService, AlertService, CacheService, DiscoveryService, InfluxService,
    PostgresService, ServerDocumentsService, ServerProcessesService, SshDeploymentService,
    TelecoService, ThresholdAIService, WebSocketService,
};
use codazzy_gateway::Result;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    info!("Codazzy Gateway v{} [{}]", env!("CARGO_PKG_VERSION"), CONFIG.environment);

    let (state, consumer, alerts) = bootstrap().await?;
    state.websocket_service.start();
    alerts.clone().start_scheduler().await;

    // uso spawn para evitar que bloquee el servidor
    let consumer_task = tokio::spawn(async move {
        if let Err(e) = consumer.start().await { error!("consumer crashed: {e}"); }
    });

    let addr: SocketAddr = format!("{}:{}", CONFIG.host, CONFIG.port)
        .parse()
        .expect("bad addr");
    info!("HTTP @ http://{addr}");

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tokio::spawn(async move {
        if let Err(e) = axum::serve(listener, create_router(state)).await {
            error!("http server: {e}");
        }
    });

    shutdown_signal().await;
    info!("shutting down...");
    consumer_task.abort();
    info!("bye");
    Ok(())
}

fn init_tracing() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| CONFIG.log_level.clone().into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();
}

async fn bootstrap() -> Result<(AppState, JetStreamConsumer, Arc<AlertService>)> {
    let influx = Arc::new(InfluxService::new()?);
    if influx.test_connection().await.is_err() {
        warn!("influx: connection test failed");
    }

    let cache = Arc::new(CacheService::new());
    let _ = cache.connect().await.map_err(|_| warn!("redis: connect failed"));

    let pg = Arc::new(PostgresService::new().await?);
    let pool = pg.pool().clone();

    let agent_conn = Arc::new(AgentConnectionService::new(pool.clone()));
    let alerts = Arc::new(AlertService::with_influx(pool.clone(), influx.clone()));

    let teleco = TelecoService::new(cache.clone());
    let _ = teleco.initialize().await.map_err(|_| warn!("teleco: init failed"));

    let ws = WebSocketService::new();
    let discovery = Arc::new(DiscoveryService::with_db(cache.clone(), pool.clone()));
    let ssh_deploy = Arc::new(SshDeploymentService::new(cache.clone()));
    let docs = Arc::new(ServerDocumentsService::new(pool.clone()));
    let processes = Arc::new(ServerProcessesService::new(pool.clone()));
    let threshold_ai = Arc::new(ThresholdAIService::new(pool, processes.clone(), docs.clone()));

    let writer = Arc::new(InfluxWriter::new());
    let mut consumer = JetStreamConsumer::with_processes_service(writer, processes.clone());
    let _ = consumer.connect().await.map_err(|_| warn!("nats: connect failed"));

    info!("services ready");
    Ok((
        AppState {
            influx_service: influx,
            cache_service: cache,
            postgres_service: pg,
            agent_connection_service: agent_conn,
            alert_service: alerts.clone(),
            teleco_service: teleco,
            websocket_service: ws,
            discovery_service: discovery,
            ssh_deployment_service: ssh_deploy,
            server_documents_service: docs,
            server_processes_service: processes,
            threshold_ai_service: threshold_ai,
        },
        consumer,
        alerts,
    ))
}

async fn shutdown_signal() {
    #[cfg(unix)]
    let terminate = async {
        signal::unix::signal(signal::unix::SignalKind::terminate())
            .expect("signal handler")
            .recv()
            .await;
    };
    #[cfg(not(unix))]
    let terminate = std::future::pending::<()>();

    tokio::select! {
        _ = signal::ctrl_c() => info!("SIGINT"),
        _ = terminate => info!("SIGTERM"),
    }
}
