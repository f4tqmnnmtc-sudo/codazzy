use axum::{body::Body, http::Request, middleware::Next, response::Response};
use std::time::Instant;
use tracing::{info, warn};

pub struct RequestLogger;

// health check
impl RequestLogger {
    pub async fn log_request(request: Request<Body>, next: Next) -> Response {
        let start = Instant::now();
        let method = request.method().clone();
        let path = request.uri().path().to_string();
        let skip = path.contains("/health") || path.contains("/ws");

        let response = next.run(request).await;

        if !skip {
            let status = response.status();
            let log = format!(
                "{} {} {} {:?}",
                method,
                path,
                status.as_u16(),
                start.elapsed()
            );
            if status.is_success() {
                info!("{}", log);
            } else {
                warn!("{}", log);
            }
        }

        response
    }
}
