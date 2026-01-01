pub mod agents;
pub mod alerts;
pub mod discovery;
pub mod documents;
pub mod health;
pub mod metrics;
pub mod predictions;
pub mod reports;
pub mod teleco;
pub mod websockets;

use crate::api::middleware::logging::RequestLogger;
use crate::services::{
    AgentConnectionService, AlertService, CacheService, DiscoveryService, InfluxService,
    PostgresService, ServerDocumentsService, ServerProcessesService, SshDeploymentService,
    TelecoService, ThresholdAIService, WebSocketService,
};
use axum::{middleware, routing::{delete, get, post}, Router};
use std::sync::Arc;
use tower_http::{compression::CompressionLayer, cors::{Any, CorsLayer}, timeout::TimeoutLayer};

#[derive(Clone)]
pub struct AppState {
    pub influx_service: Arc<InfluxService>,
    pub cache_service: Arc<CacheService>,
    pub postgres_service: Arc<PostgresService>,
    pub agent_connection_service: Arc<AgentConnectionService>,
    pub alert_service: Arc<AlertService>,
    pub teleco_service: Arc<TelecoService>,
    pub websocket_service: Arc<WebSocketService>,
    pub discovery_service: Arc<DiscoveryService>,
    pub ssh_deployment_service: Arc<SshDeploymentService>,
    pub server_documents_service: Arc<ServerDocumentsService>,
    pub server_processes_service: Arc<ServerProcessesService>,
    pub threshold_ai_service: Arc<ThresholdAIService>,
}

pub fn create_router(state: AppState) -> Router {
    let cors = CorsLayer::new().allow_origin(Any).allow_methods(Any).allow_headers(Any);

    Router::new()
        .route("/health", get(health::health_check))
        // Metrics
        .route("/api/v1/metrics/timeseries", get(metrics::timeseries))
        .route("/api/v1/metrics/agents", get(metrics::agents))
        .route("/api/v1/metrics/query", post(metrics::query))
        .route("/api/v1/metrics/cache/clear", post(metrics::clear_cache))
        // Agents
        .route("/api/v1/agents/remote-install", get(agents::list_jobs).post(agents::create_job).delete(agents::clear_jobs))
        .route("/api/v1/agents/remote-install/:job_id", get(agents::show_job).delete(agents::cancel_job))
        .route("/api/v1/agents/installed-servers", get(agents::installed_servers))
        .route("/api/v1/agents/health-check/:hostname", get(agents::ssh_health))
        .route("/api/v1/agents/uninstall", post(agents::uninstall))
        .route("/api/v1/agents/remote-config/fetch", post(agents::read_config))
        .route("/api/v1/agents/remote-config/update", post(agents::write_config))
        // Alerts
        .route("/api/v1/alerts/thresholds/analyze", post(alerts::analyze_thresholds))
        .route("/api/v1/alerts/thresholds/:device_id", get(alerts::thresholds))
        .route("/api/v1/alerts/predictions", get(alerts::predictions))
        // Teleco
        .route("/api/v1/teleco/devices", get(teleco::list_devices).post(teleco::add_device))
        .route("/api/v1/teleco/devices/:device_id", delete(teleco::remove_device))
        // Discovery
        .route("/api/v1/discovery/scan/start", post(discovery::start_scan))
        .route("/api/v1/discovery/scan/:scan_id", get(discovery::scan_status))
        .route("/api/v1/discovery/devices", get(discovery::list_devices))
        .route("/api/v1/discovery/devices/:device_id", get(discovery::device).put(discovery::update_device).delete(discovery::delete_device))
        .route("/api/v1/discovery/topology", get(discovery::topology))
        // Documents
        .route("/api/v1/servers/:node_id/documents", get(documents::list_documents).delete(documents::delete_all_documents))
        .route("/api/v1/servers/:node_id/documents/upload", post(documents::upload_document))
        .route("/api/v1/documents/:doc_id", delete(documents::delete_document))
        // Reports
        .route("/api/reports/generate", post(reports::generate_report))
        .route("/api/reports/export/:report_id", post(reports::export_report))
        // Predictions
        .route("/api/v1/predictions", post(predictions::save_predictions))
        .route("/api/v1/predictions/:node_id", get(predictions::node_predictions))
        .route("/api/v1/forecast/daily", post(predictions::forecast_daily))
        .route("/api/v1/forecast/weekly", post(predictions::forecast_weekly))
        // WebSockets
        .route("/ws", get(websockets::upgrade))
        .route("/ws/stats", get(websockets::ws_stats))
        .layer(middleware::from_fn(RequestLogger::log_request))
        .layer(CompressionLayer::new())
        .layer(cors)
        .layer(TimeoutLayer::new(std::time::Duration::from_secs(120)))
        .with_state(state)
}
