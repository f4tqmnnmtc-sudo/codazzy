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

fn default_time_range() -> String {
    "5m".to_string()
}

pub async fn timeseries(
    State(state): State<AppState>,
    Query(query): Query<TimeseriesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let measurement = query.measurement.as_deref().unwrap_or("metrics_v2");
    let field = query.field.as_deref().unwrap_or("value");
    let component = query.component.as_deref();

    let data = state
        .influx_service
        .timeseries_by_component(
            measurement,
            field,
            &query.time_range,
            query.node_id.as_deref(),
            component,
        )
        .await?;

    let count = data.as_array().map(|arr| arr.len()).unwrap_or(0);

    Ok(Json(serde_json::json!({
        "data": data,
        "metadata": {
            "measurement": measurement,
            "field": field,
            "component": component,
            "time_range": query.time_range,
            "node_id": query.node_id,
            "count": count,
            "cached": false,
            "timestamp": Utc::now().to_rfc3339()
        }
    })))
}

pub async fn clear_cache(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.influx_service.clear_cache();
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn agents(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let agents = state.influx_service.agents().await?;
    Ok(Json(serde_json::to_value(agents)?))
}

pub async fn query(
    State(_state): State<AppState>,
    Json(req): Json<QueryRequest>,
) -> Result<Json<QueryResponse>, AppError> {
    let q = req.query.trim();
    if q.is_empty() {
        return Err(AppError::Validation("La consulta no puede estar vacia".into()));
    }

    let flux = convert_simple_query_to_flux(q, &CONFIG.influx_bucket);
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
        let st = resp.status();
        let txt = resp.text().await.unwrap_or_default();
        return Err(AppError::Internal(format!("Consulta fallo: {st} - {txt}")));
    }

    let csv = resp.text().await
        .map_err(|e| AppError::Internal(format!("Error leyendo respuesta: {e}")))?;
    let data = parse_influx_csv(&csv);

    Ok(Json(QueryResponse {
        count: data.len(),
        data,
        query: q.into(),
        timestamp: Utc::now().to_rfc3339(),
    }))
}

fn convert_simple_query_to_flux(raw: &str, bucket: &str) -> String {
    // Pass-through si ya es Flux nativo
    if raw.to_lowercase().contains("from(bucket") {
        return raw.into();
    }

    let toks: Vec<&str> = raw.split_whitespace().collect();
    let upper: Vec<String> = toks.iter().map(|t| t.to_uppercase()).collect();

    let find_kw = |kw: &str| upper.iter().position(|x| x == kw);

    let measurement = find_kw("FROM")
        .and_then(|i| toks.get(i + 1).copied())
        .unwrap_or("metrics_v2");

    let time_range = find_kw("RANGE")
        .and_then(|i| toks.get(i + 1).copied())
        .map(|r| if r.starts_with('-') { r.into() } else { format!("-{r}") })
        .unwrap_or_else(|| "-1h".into());

    let limit = find_kw("LIMIT")
        .and_then(|i| toks.get(i + 1))
        .and_then(|s| s.parse::<i32>().ok())
        .unwrap_or(100);

    let mut flux = format!(
        r#"from(bucket: "{bucket}") |> range(start: {time_range}) |> filter(fn: (r) => r["_measurement"] == "{measurement}")"#
    );

    if let Some(wi) = find_kw("WHERE") {
        let end = upper.iter().skip(wi + 1)
            .position(|x| matches!(x.as_str(), "LIMIT" | "GROUP" | "ORDER"))
            .map(|i| wi + 1 + i)
            .unwrap_or(toks.len());

        let cond_str = toks[wi + 1..end].join(" ");

        for cond in cond_str.split(" AND ").chain(cond_str.split(" and ")) {
            let cond = cond.trim();
            if cond.is_empty() || cond.eq_ignore_ascii_case("AND") { continue; }

            if let Some(eq) = cond.find('=') {
                let fld = cond[..eq].trim();
                let val = cond[eq + 1..].trim().trim_matches('"');
                flux.push_str(&format!(r#" |> filter(fn: (r) => r["{fld}"] == "{val}")"#));
            }
        }
    }

    flux.push_str(&format!(r#" |> limit(n: {limit})"#));
    flux
}

fn parse_influx_csv(csv: &str) -> Vec<serde_json::Value> {
    let lns: Vec<&str> = csv.lines().collect();
    if lns.is_empty() { return vec![]; }

    // Buscar fila de headers (contiene _time, _value o _measurement)
    let (hdr_idx, hdrs) = lns.iter().enumerate()
        .filter(|(_, l)| !l.is_empty() && !l.starts_with('#'))
        .find_map(|(i, l)| {
            let cols: Vec<&str> = l.split(',').collect();
            cols.iter().any(|c| c.contains("_time") || c.contains("_value") || c.contains("_measurement"))
                .then_some((i, cols))
        })
        .unwrap_or((0, vec![]));

    if hdrs.is_empty() { return vec![]; }

    let norm_key = |h: &str| match h {
        "_time" => "time", "_value" => "value", "_measurement" => "measurement", "_field" => "field",
        x => x,
    };

    lns.iter().skip(hdr_idx + 1)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .filter_map(|l| {
            let vals: Vec<&str> = l.split(',').collect();
            if vals.len() != hdrs.len() { return None; }

            let mut row = serde_json::Map::new();
            for (i, h) in hdrs.iter().enumerate() {
                let h = h.trim();
                let v = vals.get(i).map(|x| x.trim()).unwrap_or("");
                if h.is_empty() || matches!(h, "result" | "table") { continue; }

                let k = norm_key(h);
                let jv = if matches!(k, "value" | "_value") {
                    v.parse::<f64>().map(|n| serde_json::json!(n)).unwrap_or_else(|_| serde_json::json!(v))
                } else {
                    serde_json::json!(v)
                };
                row.insert(k.into(), jv);
            }
            (!row.is_empty()).then(|| serde_json::Value::Object(row))
        })
        .collect()
}
