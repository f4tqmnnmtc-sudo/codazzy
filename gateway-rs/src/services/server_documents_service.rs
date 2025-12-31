use crate::config::CONFIG;
use crate::error::{AppError, Result};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::postgres::PgPool;
use sqlx::Row;
use tracing::{error, warn};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServerDocument {
    pub id: i32,
    pub node_id: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i32,
    pub content: String,
    #[serde(default)]
    pub summary: Option<String>,
    #[serde(default)]
    pub description: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateDocumentRequest {
    pub node_id: String,
    pub filename: String,
    pub file_type: String,
    pub file_size: i32,
    pub content: String,
    #[serde(default)]
    pub description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingResponse {
    data: Vec<EmbeddingData>,
}

#[derive(Debug, Deserialize)]
struct EmbeddingData {
    embedding: Vec<f32>,
}

pub struct ServerDocumentsService {
    pool: PgPool,
}

impl ServerDocumentsService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_document(&self, req: CreateDocumentRequest) -> Result<ServerDocument> {
        let (summary, embedding) = self
            .generate_summary_and_embedding(&req.content, &req.filename)
            .await;
        let embedding_str = embedding.map(|e| {
            format!(
                "[{}]",
                e.iter()
                    .map(|v| v.to_string())
                    .collect::<Vec<_>>()
                    .join(",")
            )
        });

        let row = sqlx::query(
            r#"INSERT INTO server_documents (node_id, filename, file_type, file_size, content, summary, embedding, description)
            VALUES ($1, $2, $3, $4, $5, $6, $7::vector, $8)
            ON CONFLICT (node_id, filename) DO UPDATE SET file_type = EXCLUDED.file_type, file_size = EXCLUDED.file_size, content = EXCLUDED.content, summary = EXCLUDED.summary, embedding = EXCLUDED.embedding, description = EXCLUDED.description, updated_at = NOW()
            RETURNING id, node_id, filename, file_type, file_size, content, summary, description, created_at, updated_at"#
        ).bind(&req.node_id).bind(&req.filename).bind(&req.file_type).bind(req.file_size).bind(&req.content).bind(&summary).bind(&embedding_str).bind(&req.description)
        .fetch_one(&self.pool).await.map_err(AppError::Database)?;

        self.row_to_document(&row)
    }

    async fn generate_summary_and_embedding(
        &self,
        content: &str,
        filename: &str,
    ) -> (Option<String>, Option<Vec<f32>>) {
        let Some(api_key) = CONFIG.openai_api_key.as_ref() else {
            warn!("OpenAI API key no configurada");
            return (None, None);
        };

        let summary = self.generate_summary(api_key, content, filename).await;
        let truncated = content[..content.len().min(8000)].to_string();
        let text = summary.as_ref().unwrap_or(&truncated);
        let embedding = self.generate_embedding(api_key, text).await;

        (summary, embedding)
    }

    async fn generate_summary(
        &self,
        api_key: &str,
        content: &str,
        filename: &str,
    ) -> Option<String> {
        let truncated = &content[..content.len().min(48000)];
        let prompt = format!(
            r#"Analiza el siguiente documento tecnico y genera un resumen estructurado.
DOCUMENTO: {}
---
{}
---
INSTRUCCIONES: 1. Extrae info relevante para sysadmin 2. Incluye: proposito, configs clave, servicios, puertos, rutas 3. Si es manual, extrae comandos importantes 4. Menos de 1500 palabras 5. Usa bullets 6. Responde en español
RESUMEN:"#,
            filename, truncated
        );

        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(120))
            .build()
            .unwrap_or_default();
        let body = serde_json::json!({"model": CONFIG.openai_model, "messages": [{"role": "system", "content": "Eres experto en documentacion tecnica IT."}, {"role": "user", "content": prompt}], "max_completion_tokens": 2000, "reasoning_effort": "low"});

        match client
            .post("https://api.openai.com/v1/chat/completions")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => {
                resp.json::<serde_json::Value>().await.ok().and_then(|j| {
                    j["choices"][0]["message"]["content"]
                        .as_str()
                        .map(String::from)
                })
            }
            Ok(resp) => {
                error!(
                    "OpenAI API error: {}",
                    resp.text().await.unwrap_or_default()
                );
                None
            }
            Err(e) => {
                error!("Error llamando OpenAI: {}", e);
                None
            }
        }
    }

    async fn generate_embedding(&self, api_key: &str, text: &str) -> Option<Vec<f32>> {
        let truncated = &text[..text.len().min(32000)];
        let client = reqwest::Client::builder()
            .timeout(std::time::Duration::from_secs(60))
            .build()
            .unwrap_or_default();
        let body = serde_json::json!({"model": "text-embedding-3-small", "input": truncated});

        match client
            .post("https://api.openai.com/v1/embeddings")
            .header("Authorization", format!("Bearer {}", api_key))
            .header("Content-Type", "application/json")
            .json(&body)
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => resp
                .json::<EmbeddingResponse>()
                .await
                .ok()
                .and_then(|r| r.data.first().map(|d| d.embedding.clone())),
            Ok(resp) => {
                error!(
                    "OpenAI embedding error: {}",
                    resp.text().await.unwrap_or_default()
                );
                None
            }
            Err(e) => {
                error!("Error generando embedding: {}", e);
                None
            }
        }
    }

    pub async fn documents(&self, node_id: &str) -> Result<Vec<ServerDocument>> {
        let rows = sqlx::query("SELECT id, node_id, filename, file_type, file_size, content, summary, description, created_at, updated_at FROM server_documents WHERE node_id = $1 ORDER BY created_at DESC")
            .bind(node_id).fetch_all(&self.pool).await.map_err(AppError::Database)?;
        rows.iter().map(|r| self.row_to_document(r)).collect()
    }

    pub async fn document(&self, doc_id: i32) -> Result<Option<ServerDocument>> {
        sqlx::query("SELECT id, node_id, filename, file_type, file_size, content, summary, description, created_at, updated_at FROM server_documents WHERE id = $1")
            .bind(doc_id).fetch_optional(&self.pool).await.map_err(AppError::Database)?
            .map(|r| self.row_to_document(&r)).transpose()
    }

    pub async fn delete_document(&self, doc_id: i32) -> Result<bool> {
        let result = sqlx::query("DELETE FROM server_documents WHERE id = $1")
            .bind(doc_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn delete_all_documents(&self, node_id: &str) -> Result<i64> {
        let result = sqlx::query("DELETE FROM server_documents WHERE node_id = $1")
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(result.rows_affected() as i64)
    }

    pub async fn summaries_for_ai(&self, node_id: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query("SELECT filename, file_type, summary, description FROM server_documents WHERE node_id = $1 AND summary IS NOT NULL ORDER BY created_at DESC")
            .bind(node_id).fetch_all(&self.pool).await.map_err(AppError::Database)?;
        Ok(rows.iter().filter_map(|r| {
            r.get::<Option<String>, _>("summary").map(|sum| serde_json::json!({"filename": r.get::<String, _>("filename"), "type": r.get::<String, _>("file_type"), "description": r.get::<Option<String>, _>("description"), "summary": sum}))
        }).collect())
    }

    pub async fn documents_for_ai(&self, node_id: &str) -> Result<Vec<serde_json::Value>> {
        let rows = sqlx::query("SELECT filename, file_type, COALESCE(summary, LEFT(content, 5000)) as content, description FROM server_documents WHERE node_id = $1 ORDER BY created_at DESC")
            .bind(node_id).fetch_all(&self.pool).await.map_err(AppError::Database)?;
        Ok(rows.iter().map(|r| serde_json::json!({"filename": r.get::<String, _>("filename"), "type": r.get::<String, _>("file_type"), "description": r.get::<Option<String>, _>("description"), "content": r.get::<String, _>("content")})).collect())
    }

    pub async fn search_similar_documents(
        &self,
        query: &str,
        node_id: Option<&str>,
        limit: i32,
    ) -> Result<Vec<ServerDocument>> {
        let api_key = CONFIG
            .openai_api_key
            .as_ref()
            .ok_or_else(|| AppError::Configuration("OpenAI API key no configurada".into()))?;
        let embedding = self
            .generate_embedding(api_key, query)
            .await
            .ok_or_else(|| AppError::Internal("Error generando embedding".into()))?;
        let embedding_str = format!(
            "[{}]",
            embedding
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        );

        let rows = if let Some(nid) = node_id {
            sqlx::query("SELECT id, node_id, filename, file_type, file_size, content, summary, description, created_at, updated_at, embedding <=> $1::vector as distance FROM server_documents WHERE node_id = $2 AND embedding IS NOT NULL ORDER BY distance LIMIT $3")
                .bind(&embedding_str).bind(nid).bind(limit).fetch_all(&self.pool).await.map_err(AppError::Database)?
        } else {
            sqlx::query("SELECT id, node_id, filename, file_type, file_size, content, summary, description, created_at, updated_at, embedding <=> $1::vector as distance FROM server_documents WHERE embedding IS NOT NULL ORDER BY distance LIMIT $2")
                .bind(&embedding_str).bind(limit).fetch_all(&self.pool).await.map_err(AppError::Database)?
        };

        rows.iter().map(|r| self.row_to_document(r)).collect()
    }

    pub async fn regenerate_summaries(&self, node_id: &str) -> Result<i32> {
        let docs = self.documents(node_id).await?;
        let mut count = 0;
        for doc in docs {
            if self
                .create_document(CreateDocumentRequest {
                    node_id: doc.node_id,
                    filename: doc.filename,
                    file_type: doc.file_type,
                    file_size: doc.file_size,
                    content: doc.content,
                    description: doc.description,
                })
                .await
                .is_ok()
            {
                count += 1;
            }
        }
        Ok(count)
    }

    fn row_to_document(&self, r: &sqlx::postgres::PgRow) -> Result<ServerDocument> {
        Ok(ServerDocument {
            id: r.get("id"),
            node_id: r.get("node_id"),
            filename: r.get("filename"),
            file_type: r.get("file_type"),
            file_size: r.get("file_size"),
            content: r.get("content"),
            summary: r.get("summary"),
            description: r.get("description"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
        })
    }
}
