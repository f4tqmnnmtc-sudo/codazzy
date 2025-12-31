use axum::{http::StatusCode, response::{IntoResponse, Response}, Json};
use serde_json::json;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("db: {0}")]
    Database(#[from] sqlx::Error),
    #[error("redis: {0}")]
    Redis(#[from] redis::RedisError),
    #[error("influx: {0}")]
    InfluxDb(String),
    #[error("nats: {0}")]
    Nats(String),
    #[error("nats sub: {0}")]
    NatsSubscription(String),
    #[error("json: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("msgpack: {0}")]
    MessagePack(String),
    #[error("lz4: {0}")]
    Lz4Decompression(String),
    #[error("http: {0}")]
    HttpClient(#[from] reqwest::Error),
    #[error("validation: {0}")]
    Validation(String),
    #[error("auth: {0}")]
    Authentication(String),
    #[error("authz: {0}")]
    Authorization(String),
    #[error("not found: {0}")]
    NotFound(String),
    #[error("exists: {0}")]
    AlreadyExists(String),
    #[error("conflict: {0}")]
    Conflict(String),
    #[error("unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("timeout: {0}")]
    Timeout(String),
    #[error("snmp: {0}")]
    Snmp(String),
    #[error("ssh: {0}")]
    Ssh(String),
    #[error("mqtt: {0}")]
    Mqtt(String),
    #[error("config: {0}")]
    Configuration(String),
    #[error("internal: {0}")]
    Internal(String),
    #[error("io: {0}")]
    Io(#[from] std::io::Error),
    #[error("{0}")]
    Other(#[from] anyhow::Error),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (code, etype) = self.status_info();
        (code, Json(json!({"error": {"type": etype, "message": self.user_msg()}}))).into_response()
    }
}

impl AppError {
    fn status_info(&self) -> (StatusCode, &'static str) {
        match self {
            Self::Validation(_) | Self::MessagePack(_) | Self::Lz4Decompression(_) =>
                (StatusCode::BAD_REQUEST, "validation"),
            Self::Authentication(_) => (StatusCode::UNAUTHORIZED, "auth"),
            Self::Authorization(_) => (StatusCode::FORBIDDEN, "authz"),
            Self::NotFound(_) => (StatusCode::NOT_FOUND, "notfound"),
            Self::AlreadyExists(_) | Self::Conflict(_) => (StatusCode::CONFLICT, "conflict"),
            Self::ServiceUnavailable(_) | Self::Nats(_) | Self::NatsSubscription(_) =>
                (StatusCode::SERVICE_UNAVAILABLE, "unavailable"),
            Self::Timeout(_) => (StatusCode::GATEWAY_TIMEOUT, "timeout"),
            Self::Database(_) => (StatusCode::INTERNAL_SERVER_ERROR, "db"),
            Self::Redis(_) => (StatusCode::INTERNAL_SERVER_ERROR, "cache"),
            Self::InfluxDb(_) => (StatusCode::INTERNAL_SERVER_ERROR, "influx"),
            Self::Serialization(_) => (StatusCode::INTERNAL_SERVER_ERROR, "json"),
            Self::HttpClient(_) => (StatusCode::INTERNAL_SERVER_ERROR, "http"),
            Self::Snmp(_) | Self::Ssh(_) | Self::Mqtt(_) =>
                (StatusCode::INTERNAL_SERVER_ERROR, "proto"),
            Self::Configuration(_) => (StatusCode::INTERNAL_SERVER_ERROR, "cfg"),
            _ => (StatusCode::INTERNAL_SERVER_ERROR, "internal"),
        }
    }

    fn user_msg(&self) -> String {
        match self {
            Self::Validation(m) | Self::MessagePack(m) | Self::Lz4Decompression(m) => m.clone(),
            Self::Authentication(m) | Self::Authorization(m) => m.clone(),
            Self::NotFound(m) | Self::AlreadyExists(m) | Self::Conflict(m) => m.clone(),
            Self::ServiceUnavailable(m) | Self::Nats(m) | Self::NatsSubscription(m) => m.clone(),
            Self::Timeout(m) | Self::Snmp(m) | Self::Ssh(m) | Self::Mqtt(m) => m.clone(),
            Self::Configuration(m) => m.clone(),
            Self::Database(_) => "database error".into(),
            Self::Redis(_) => "cache error".into(),
            Self::InfluxDb(_) => "timeseries error".into(),
            Self::Serialization(_) => "serialization error".into(),
            Self::HttpClient(_) => "http client error".into(),
            _ => "internal error".into(),
        }
    }
}

pub type Result<T> = std::result::Result<T, AppError>;

pub trait IntoAppError<T> {
    fn ctx(self, c: &str) -> Result<T>;
}

impl<T, E: std::fmt::Display> IntoAppError<T> for std::result::Result<T, E> {
    fn ctx(self, c: &str) -> Result<T> {
        self.map_err(|e| AppError::Internal(format!("{c}: {e}")))
    }
}
