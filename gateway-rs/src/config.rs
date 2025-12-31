use once_cell::sync::Lazy;
use std::env;

pub static CONFIG: Lazy<Config> = Lazy::new(|| Config::load().expect("config load failed"));

#[derive(Debug, Clone)]
pub struct Config {
    pub host: String,
    pub port: u16,
    pub debug: bool,
    pub environment: String,
    pub influx_url: String,
    pub influx_token: String,
    pub influx_org: String,
    pub influx_bucket: String,
    pub influx_ml_bucket: String,
    pub redis_url: String,
    pub cache_ttl: u64,
    pub nats_url: String,
    pub stream_name: String,
    pub consumer_name: String,
    pub postgres_host: String,
    pub postgres_port: u16,
    pub postgres_db: String,
    pub postgres_user: String,
    pub postgres_password: String,
    pub postgres_min_pool_size: u32,
    pub postgres_max_pool_size: u32,
    pub database_url: String,
    pub secret_key: String,
    pub algorithm: String,
    pub access_token_expire_minutes: u64,
    pub openai_api_key: Option<String>,
    pub openai_model: String,
    pub openai_max_tokens: u32,
    pub openai_temperature: f32,
    pub allowed_origins: Vec<String>,
    pub allowed_methods: Vec<String>,
    pub allowed_headers: Vec<String>,
    pub batch_size: usize,
    pub fetch_batch_size: usize,
    pub consumer_workers: usize,
    pub influx_flush_interval: u64,
    pub log_level: String,
    // Timeouts y duraciones
    pub backpressure_timeout_ms: u64,
    pub discovery_scan_timeout_secs: u64,
    pub discovery_ping_timeout_secs: u64,
    pub nats_stream_max_age_days: u64,
    pub nats_duplicate_window_secs: u64,
    pub nats_ack_wait_secs: u64,
    pub prediction_interval_secs: u64,
    pub prediction_enabled: bool,
}

impl Config {
    #[inline] fn env_var(k: &str, d: &str) -> String { env::var(k).unwrap_or_else(|_| d.into()) }
    #[inline] fn env_parse<T: std::str::FromStr>(k: &str, d: T) -> T { env::var(k).ok().and_then(|v| v.parse().ok()).unwrap_or(d) }
    #[inline] fn env_list(k: &str, d: &str) -> Vec<String> { Self::env_var(k, d).split(',').map(str::trim).map(Into::into).collect() }

    pub fn load() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        let pg_h = Self::env_var("POSTGRES_HOST", "postgres");
        let pg_p = Self::env_parse("POSTGRES_PORT", 5432u16);
        let pg_d = Self::env_var("POSTGRES_DB", "codazzy");
        let pg_u = Self::env_var("POSTGRES_USER", "codazzy");
        let pg_pw = Self::env_var("POSTGRES_PASSWORD", "codazzy");

        let db_url = env::var("DATABASE_URL")
            .unwrap_or_else(|_| format!("postgresql://{pg_u}:{pg_pw}@{pg_h}:{pg_p}/{pg_d}"));

        Ok(Config {
            host: Self::env_var("HOST", "0.0.0.0"),
            port: Self::env_parse("PORT", 8000),
            debug: Self::env_parse("DEBUG", false),
            environment: Self::env_var("ENVIRONMENT", "production"),
            influx_url: Self::env_var("INFLUX_URL", "http://influxdb:8086"),
            influx_token: Self::env_var("INFLUX_TOKEN", ""),
            influx_org: Self::env_var("INFLUX_ORG", "monitoring"),
            influx_bucket: Self::env_var("INFLUX_BUCKET", "metrics"),
            influx_ml_bucket: Self::env_var("INFLUX_ML_BUCKET", "ml_features"),
            redis_url: Self::env_var("REDIS_URL", "redis://redis:6379"),
            cache_ttl: Self::env_parse("CACHE_TTL", 300),
            nats_url: Self::env_var("NATS_URL", "nats://nats:4222"),
            stream_name: Self::env_var("STREAM_NAME", "METRICS"),
            consumer_name: Self::env_var("CONSUMER_NAME", "metrics-processor"),
            postgres_host: pg_h,
            postgres_port: pg_p,
            postgres_db: pg_d,
            postgres_user: pg_u,
            postgres_password: pg_pw,
            postgres_min_pool_size: Self::env_parse("POSTGRES_MIN_POOL_SIZE", 5),
            postgres_max_pool_size: Self::env_parse("POSTGRES_MAX_POOL_SIZE", 20),
            database_url: db_url,
            secret_key: Self::env_var("SECRET_KEY", "change-me-in-production"),
            algorithm: Self::env_var("ALGORITHM", "HS256"),
            access_token_expire_minutes: Self::env_parse("ACCESS_TOKEN_EXPIRE_MINUTES", 30),
            openai_api_key: env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
            openai_model: Self::env_var("OPENAI_MODEL", "gpt-5-nano"),
            openai_max_tokens: Self::env_parse("OPENAI_MAX_TOKENS", 16000),
            openai_temperature: Self::env_parse("OPENAI_TEMPERATURE", 0.1),
            allowed_origins: Self::env_list("ALLOWED_ORIGINS", "*"),
            allowed_methods: Self::env_list("ALLOWED_METHODS", "GET,POST,PUT,DELETE,OPTIONS"),
            allowed_headers: Self::env_list("ALLOWED_HEADERS", "*"),
            batch_size: Self::env_parse("BATCH_SIZE", 500),
            fetch_batch_size: Self::env_parse("FETCH_BATCH_SIZE", 1000),
            consumer_workers: Self::env_parse("CONSUMER_WORKERS", 16),
            influx_flush_interval: Self::env_parse("INFLUX_BATCH_FLUSH_INTERVAL_MS", 100),
            log_level: Self::env_var("RUST_LOG", "info,codazzy_gateway=debug"),
            // Timeouts y duraciones
            backpressure_timeout_ms: Self::env_parse("BACKPRESSURE_TIMEOUT_MS", 100),
            discovery_scan_timeout_secs: Self::env_parse("DISCOVERY_SCAN_TIMEOUT_SECS", 120),
            discovery_ping_timeout_secs: Self::env_parse("DISCOVERY_PING_TIMEOUT_SECS", 2),
            nats_stream_max_age_days: Self::env_parse("NATS_STREAM_MAX_AGE_DAYS", 7),
            nats_duplicate_window_secs: Self::env_parse("NATS_DUPLICATE_WINDOW_SECS", 120),
            nats_ack_wait_secs: Self::env_parse("NATS_ACK_WAIT_SECS", 30),
            prediction_interval_secs: Self::env_parse("PREDICTION_INTERVAL_SECS", 30),
            prediction_enabled: Self::env_parse("PREDICTION_ENABLED", true),
        })
    }

    #[inline] pub fn postgres_dsn(&self) -> String { self.database_url.clone() }
    #[inline] pub fn is_debug(&self) -> bool { self.debug || self.environment == "development" }
    #[inline] pub fn has_openai(&self) -> bool { self.openai_api_key.is_some() }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults() {
        env::remove_var("PORT");
        env::remove_var("HOST");
        let c = Config::load().unwrap();
        assert_eq!(c.port, 8000);
        assert_eq!(c.host, "0.0.0.0");
    }
}
