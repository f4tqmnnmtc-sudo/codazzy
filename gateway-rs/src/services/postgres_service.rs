use crate::config::CONFIG;
use crate::error::{AppError, Result};
use sqlx::postgres::{PgPool, PgPoolOptions};
use std::time::Duration;
use tracing::info;

const MIGRATIONS: &[(&str, &str)] = &[
    ("001_init", include_str!("../../migrations/001_init.sql")),
    ("002_extensions", include_str!("../../migrations/002_extensions.sql")),
    ("003_server_documents", include_str!("../../migrations/003_server_documents.sql")),
    ("004_indexes", include_str!("../../migrations/004_indexes.sql")),
];

pub struct PostgresService {
    pool: PgPool,
}

impl PostgresService {
    pub async fn new() -> Result<Self> {
        let pool = PgPoolOptions::new()
            .min_connections(CONFIG.postgres_min_pool_size)
            .max_connections(CONFIG.postgres_max_pool_size)
            .acquire_timeout(Duration::from_secs(30))
            .idle_timeout(Duration::from_secs(600))
            .connect(&CONFIG.database_url)
            .await
            .map_err(AppError::Database)?;

        info!("pg pool connected");
        let svc = Self { pool };
        svc.migrate().await?;
        Ok(svc)
    }

    #[inline]
    pub fn pool(&self) -> &PgPool { &self.pool }

    pub async fn test_connection(&self) -> Result<bool> {
        let _: (i32,) = sqlx::query_as("SELECT 1")
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(true)
    }

    async fn migrate(&self) -> Result<()> {
        self.ensure_migrations_table().await?;

        for (name, sql) in MIGRATIONS {
            if !self.migration_applied(name).await? {
                self.run_migration(name, sql).await?;
            }
        }

        info!("migrations done");
        Ok(())
    }

    async fn ensure_migrations_table(&self) -> Result<()> {
        sqlx::query(
            r#"CREATE TABLE IF NOT EXISTS _migrations (
                id SERIAL PRIMARY KEY,
                name VARCHAR(255) UNIQUE NOT NULL,
                applied_at TIMESTAMPTZ DEFAULT NOW()
            )"#
        )
        .execute(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(())
    }

    async fn migration_applied(&self, name: &str) -> Result<bool> {
        let row: Option<(i32,)> = sqlx::query_as(
            "SELECT 1 FROM _migrations WHERE name = $1"
        )
        .bind(name)
        .fetch_optional(&self.pool)
        .await
        .map_err(AppError::Database)?;
        Ok(row.is_some())
    }

    async fn run_migration(&self, name: &str, sql: &str) -> Result<()> {
        info!("applying migration: {name}");

        // Ejecutar cada statement por separado (separados por ;)
        for stmt in sql.split(';').map(|s| s.trim()).filter(|s| !s.is_empty()) {
            if let Err(e) = sqlx::query(stmt).execute(&self.pool).await {
                // TODO Ignorar errores (darle una vuelta a esto)
                let err_str = e.to_string();
                if !err_str.contains("already exists") && !err_str.contains("duplicate key") {
                    tracing::warn!("migration {name} stmt error (continuing): {e}");
                }
            }
        }

        sqlx::query("INSERT INTO _migrations (name) VALUES ($1) ON CONFLICT DO NOTHING")
            .bind(name)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;

        Ok(())
    }

    pub fn pool_stats(&self) -> PoolStats {
        PoolStats {
            size: self.pool.size(),
            idle: self.pool.num_idle(),
            min_connections: CONFIG.postgres_min_pool_size,
            max_connections: CONFIG.postgres_max_pool_size,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize)]
pub struct PoolStats {
    pub size: u32,
    pub idle: usize,
    pub min_connections: u32,
    pub max_connections: u32,
}
