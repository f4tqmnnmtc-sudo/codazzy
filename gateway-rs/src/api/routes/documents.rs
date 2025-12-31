use axum::{
    extract::{Path, State},
    Json,
};
use serde::Deserialize;

use crate::api::routes::AppState;
use crate::error::AppError;
use crate::services::server_documents_service::CreateDocumentRequest;

const MAX_FILE_SIZE: i32 = 15 * 1024 * 1024;
const ALLOWED_TYPES: &[&str] = &[".txt", ".json"];

#[derive(Debug, Deserialize)]
pub struct UploadDocumentRequest {
    pub filename: String,
    pub file_type: String,
    pub file_size: i32,
    pub content: String,
    pub description: Option<String>,
}

pub async fn list_documents(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let documents = state
        .server_documents_service
        .documents(&node_id)
        .await?;

    Ok(Json(serde_json::json!({
        "success": true,
        "node_id": node_id,
        "documents": documents,
        "total": documents.len()
    })))
}

pub async fn upload_document(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
    Json(request): Json<UploadDocumentRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if request.file_size > MAX_FILE_SIZE {
        return Err(AppError::Validation(
            "El archivo excede el limite de 1MB".to_string(),
        ));
    }

    if !ALLOWED_TYPES.contains(&request.file_type.as_str()) {
        return Err(AppError::Validation(format!(
            "Tipo de archivo '{}' no permitido",
            request.file_type
        )));
    }

    let document = state
        .server_documents_service
        .create_document(CreateDocumentRequest {
            node_id: node_id.clone(),
            filename: request.filename,
            file_type: request.file_type,
            file_size: request.file_size,
            content: request.content,
            description: request.description,
        })
        .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "document": document }),
    ))
}

pub async fn delete_document(
    State(state): State<AppState>,
    Path(doc_id): Path<i32>,
) -> Result<Json<serde_json::Value>, AppError> {
    if state
        .server_documents_service
        .delete_document(doc_id)
        .await?
    {
        Ok(Json(serde_json::json!({ "success": true })))
    } else {
        Err(AppError::NotFound(format!(
            "Documento {} no encontrado",
            doc_id
        )))
    }
}

pub async fn delete_all_documents(
    State(state): State<AppState>,
    Path(node_id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let count = state
        .server_documents_service
        .delete_all_documents(&node_id)
        .await?;

    Ok(Json(
        serde_json::json!({ "success": true, "deleted_count": count }),
    ))
}
