use axum::{
    extract::{Query, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

use crate::api::routes::AppState;
use crate::config::CONFIG;
use crate::error::AppError;

#[derive(Debug, Deserialize)]
pub struct TimeseriesQuery {
    pub measurement: Option<String>,
    pub field: Option<String>,
    pub component: Option<String>,
    #[serde(default = "default_time_range")]
    pub time_range: String,
    pub node_id: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct QueryRequest {
    pub query: String,
}

#[derive(Debug, Serialize)]
pub struct QueryResponse {
    pub data: Vec<serde_json::Value>,
    pub count: usize,
    pub query: String,
    pub timestamp: String,
}

fn default_time_range() -> String { "5m".into() }

pub async fn timeseries(
    State(st): State<AppState>,
    Query(q): Query<TimeseriesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let meas = q.measurement.as_deref().unwrap_or("metrics_v2");
    let fld = q.field.as_deref().unwrap_or("value");

    let data = st.influx_service
        .timeseries_by_component(meas, fld, &q.time_range, q.node_id.as_deref(), q.component.as_deref())
        .await?;

    let n = data.as_array().map_or(0, |a| a.len());

    Ok(Json(serde_json::json!({
        "data": data,
        "metadata": {
            "measurement": meas, "field": fld, "component": q.component,
            "time_range": q.time_range, "node_id": q.node_id,
            "count": n, "cached": false, "timestamp": Utc::now().to_rfc3339()
        }
    })))
}

pub async fn clear_cache(State(st): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    st.influx_service.clear_cache();
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn agents(State(st): State<AppState>) -> Result<Json<serde_json::Value>, AppError> {
    let list = st.influx_service.agents().await?;
    Ok(Json(serde_json::to_value(list)?))
}

pub async fn query(
    State(_st): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    let raw = req.query.trim();
    if raw.is_empty() {
        return Err(AppError::Validation("La consulta no puede estar vacia".into()));
    }

    let flux = to_flux(raw, &CONFIG.influx_bucket);
    let url = format!("{}/api/v2/query?org={}", CONFIG.influx_url, CONFIG.influx_org);

    let resp = reqwest::Client::new()
        .post(&url)
        .header("Authorization", format!("Token {}", CONFIG.influx_token))
        .header("Content-Type", "application/vnd.flux")
        .header("Accept", "application/csv")
        .body(flux)
        .send()
        .await
        .map_err(|e| AppError::Internal(format!("Error en peticion a InfluxDB: {e}")))?;

    if !resp.status().is_success() {
        let (st, body) = (resp.status(), resp.text().await.unwrap_or_default());
        return Err(AppError::Internal(format!("Consulta fallo: {st} - {body}")));
    }

    let csv = resp.text().await
        .map_err(|e| AppError::Internal(format!("Error leyendo respuesta: {e}")))?;

    let rows = parse_csv(&csv);
    Ok(Json(QueryResponse {
        count: rows.len(),
        data: rows,
        query: raw.into(),
        timestamp: Utc::now().to_rfc3339(),
    }))
}

// Pass-through si ya es Flux nativo
fn to_flux(raw: &str, bucket: &str) -> String {
    if raw.to_lowercase().contains("from(bucket") { return raw.into(); }

    let toks: Vec<&str> = raw.split_whitespace().collect();
    let upper: Vec<String> = toks.iter().map(|t| t.to_uppercase()).collect();
    let kw = |k: &str| upper.iter().position(|x| x == k);

    let meas = kw("FROM").and_then(|i| toks.get(i + 1).copied()).unwrap_or("metrics_v2");

    let range = kw("RANGE")
        .and_then(|i| toks.get(i + 1).copied())
        .map(|r| if r.starts_with('-') { r.into() } else { format!("-{r}") })
        .unwrap_or_else(|| "-1h".into());

    let lim = kw("LIMIT")
        .and_then(|i| toks.get(i + 1))
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(100);

    let mut out = format!(
        r#"from(bucket: "{bucket}") |> range(start: {range}) |> filter(fn: (r) => r["_measurement"] == "{meas}")"#
    );

    if let Some(wi) = kw("WHERE") {
        let stop = upper.iter().skip(wi + 1)
            .position(|x| matches!(x.as_str(), "LIMIT" | "GROUP" | "ORDER"))
            .map_or(toks.len(), |p| wi + 1 + p);

        let conds = toks[wi + 1..stop].join(" ");
        for c in conds.split(" AND ").chain(conds.split(" and ")) {
            let c = c.trim();
            if c.is_empty() || c.eq_ignore_ascii_case("AND") { continue; }
            if let Some(eq) = c.find('=') {
                let (f, v) = (c[..eq].trim(), c[eq + 1..].trim().trim_matches('"'));
                out.push_str(&format!(r#" |> filter(fn: (r) => r["{f}"] == "{v}")"#));
            }
        }
    }
    out.push_str(&format!(r#" |> limit(n: {lim})"#));
    out
}

// Buscar fila de headers (contiene _time, _value o _measurement)
fn parse_csv(csv: &str) -> Vec<serde_json::Value> {
    let lines: Vec<&str> = csv.lines().collect();
    if lines.is_empty() { return vec![]; }

    let (hdr_i, hdrs) = lines.iter().enumerate()
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .find_map(|(i, l)| {
            let cols: Vec<&str> = l.split(',').collect();
            cols.iter().any(|c| c.contains("_time") || c.contains("_value") || c.contains("_measurement"))
                .then_some((i, cols))
        })
        .unwrap_or((0, vec![]));

    if hdrs.is_empty() { return vec![]; }

    fn norm(h: &str) -> &str {
        match h {
            "_time" => "time", "_value" => "value",
            "_measurement" => "measurement", "_field" => "field",
            x => x,
        }
    }

    lines.iter().skip(hdr_i + 1)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let vals: Vec<&str> = l.split(',').collect();
            if vals.len() != hdrs.len() { return None; }

            let mut row = serde_json::Map::new();
            for (i, h) in hdrs.iter().enumerate() {
                let h = h.trim();
                let v = vals.get(i).map_or("", |x| x.trim());
                if h.is_empty() || matches!(h, "result" | "table") { continue; }

                let k = norm(h);
                let jv = if matches!(k, "value" | "_value") {
                    v.parse::<f64>().map_or_else(|_| serde_json::json!(v), |n| serde_json::json!(n))
                } else {
                    serde_json::json!(v)
                };
                row.insert(k.into(), jv);
            }
            (!row.is_empty()).then(|| serde_json::Value::Object(row))
        })
        .collect()
}
