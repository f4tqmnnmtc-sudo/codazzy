use std::collections::HashMap;

use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Deserialize;

use crate::api::routes::AppState;
use crate::error::AppError;
use crate::models::discovery::StartScanRequest;
use crate::services::DiscoveredDevice;

#[derive(Debug, Deserialize)]
pub struct DevicesQuery {
    pub scan_id: Option<String>,
    pub status: Option<String>,
    pub device_type: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct UpdateDeviceRequest {
    pub hostname: Option<String>,
    pub vendor: Option<String>,
    pub device_type: Option<String>,
    pub os: Option<String>,
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct TopologyQuery {
    pub algorithm: Option<String>,
}

async fn cached_device(
    state: &AppState,
    device_id: &str,
) -> Result<serde_json::Value, AppError> {
    state
        .cache_service
        .hget("discovery:devices", device_id)
        .await?
        .ok_or_else(|| AppError::NotFound(format!("Device not found: {}", device_id)))
}

fn topology_json(
    devices: &[DiscoveredDevice],
    scan_id: String,
    include_type_counts: bool,
) -> serde_json::Value {
    if devices.is_empty() {
        return serde_json::json!({
            "scan_id": scan_id,
            "nodes": [],
            "edges": [],
            "statistics": { "total_nodes": 0, "total_edges": 0, "device_type_counts": {} },
            "generated_at": Utc::now().to_rfc3339()
        });
    }

    let gateway_idx = devices
        .iter()
        .position(|d| {
            d.device_type == "router" || d.device_type == "gateway" || d.ip_address.ends_with(".1")
        })
        .unwrap_or(0);

    let gateway = &devices[gateway_idx];
    let other_devices: Vec<_> = devices
        .iter()
        .enumerate()
        .filter(|(i, _)| *i != gateway_idx)
        .map(|(_, d)| d)
        .collect();

    const CENTER_X: f64 = 400.0;
    const CENTER_Y: f64 = 300.0;
    const RADIUS: f64 = 250.0;

    let mut nodes = vec![serde_json::json!({
        "id": gateway.id,
        "ip_address": gateway.ip_address,
        "hostname": gateway.hostname,
        "device_type": if gateway.device_type == "unknown" { "router" } else { &gateway.device_type },
        "status": gateway.status,
        "position": { "x": CENTER_X, "y": CENTER_Y }
    })];

    let num_others = other_devices.len().max(1);
    for (i, device) in other_devices.iter().enumerate() {
        let angle = (2.0 * std::f64::consts::PI * i as f64) / num_others as f64;
        nodes.push(serde_json::json!({
            "id": device.id,
            "ip_address": device.ip_address,
            "hostname": device.hostname,
            "device_type": device.device_type,
            "status": device.status,
            "position": { "x": CENTER_X + RADIUS * angle.cos(), "y": CENTER_Y + RADIUS * angle.sin() }
        }));
    }

    let edges: Vec<_> = other_devices
        .iter()
        .map(|device| {
            serde_json::json!({
                "id": format!("edge-{}-{}", gateway.id, device.id),
                "source": gateway.id,
                "target": device.id,
                "source_id": gateway.id,
                "target_id": device.id,
                "type": "default",
                "connection_type": "physical"
            })
        })
        .collect();

    let device_type_counts: HashMap<String, usize> = if include_type_counts {
        devices.iter().fold(HashMap::new(), |mut acc, d| {
            let dtype = if d.ip_address.ends_with(".1") && d.device_type == "unknown" {
                "router".to_string()
            } else {
                d.device_type.clone()
            };
            *acc.entry(dtype).or_insert(0) += 1;
            acc
        })
    } else {
        HashMap::new()
    };

    serde_json::json!({
        "scan_id": scan_id,
        "nodes": nodes,
        "edges": edges,
        "statistics": {
            "total_nodes": nodes.len(),
            "total_edges": edges.len(),
            "device_type_counts": device_type_counts
        },
        "generated_at": Utc::now().to_rfc3339()
    })
}

pub async fn start_scan(
    State(state): State<AppState>,
    Json(request): Json<StartScanRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let scan_id = state
        .discovery_service
        .start_scan(request.target_ranges.clone())
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "scan_id": scan_id,
        "status": "running",
        "target_ranges": request.target_ranges
    })))
}

pub async fn scan_status(
    State(state): State<AppState>,
    Path(scan_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .discovery_service
        .scan_status(&scan_id)
        .await?
        .map(|s| Json(serde_json::to_value(s).unwrap()))
        .ok_or_else(|| AppError::NotFound(format!("Scan not found: {}", scan_id)))
}

pub async fn list_devices(
    State(state): State<AppState>,
    Query(params): Query<DevicesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let devices = state
        .discovery_service
        .discovered_devices(params.scan_id.as_deref())
        .await?;

    let (by_status, by_type) = devices.iter().fold(
        (HashMap::<String, u32>::new(), HashMap::<String, u32>::new()),
        |(mut status, mut dtype), d| {
            *status.entry(d.status.clone()).or_insert(0) += 1;
            *dtype.entry(d.device_type.clone()).or_insert(0) += 1;
            (status, dtype)
        },
    );

    Ok(Json(serde_json::json!({
        "success": true,
        "devices": devices,
        "summary": { "total": devices.len(), "by_status": by_status, "by_type": by_type }
    })))
}

pub async fn device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    Ok(Json(cached_device(&state, &device_id).await?))
}

pub async fn update_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
    Json(request): Json<UpdateDeviceRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut device = cached_device(&state, &device_id).await?;

    if let Some(v) = &request.hostname {
        device["hostname"] = serde_json::json!(v);
    }
    if let Some(v) = &request.vendor {
        device["vendor"] = serde_json::json!(v);
    }
    if let Some(v) = &request.device_type {
        device["device_type"] = serde_json::json!(v);
    }
    if let Some(v) = &request.os {
        device["os"] = serde_json::json!(v);
    }
    if let Some(v) = &request.description {
        device["description"] = serde_json::json!(v);
    }

    state
        .cache_service
        .hset("discovery:devices", &device_id, &device)
        .await?;
    state
        .cache_service
        .set_ex(&format!("discovery:device:{}", device_id), &device, 86400)
        .await?;

    let _ = state
        .discovery_service
        .update_device_info(
            &device_id,
            request.hostname,
            request.vendor,
            request.device_type,
            request.os,
            request.description,
        )
        .await;

    Ok(Json(
        serde_json::json!({ "success": true, "device": device }),
    ))
}

pub async fn delete_device(
    State(state): State<AppState>,
    Path(device_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state
        .cache_service
        .hdel("discovery:devices", &device_id)
        .await?;
    state
        .cache_service
        .delete(&format!("discovery:device:{}", device_id))
        .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "device_id": device_id }),
    ))
}

pub async fn topology(
    State(state): State<AppState>,
    Query(_params): Query<TopologyQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let devices = state.discovery_service.discovered_devices(None).await?;

    let scan_id = devices
        .first()
        .map(|d| d.scan_id.clone())
        .unwrap_or_default();

    Ok(Json(topology_json(&devices, scan_id, true)))
}
