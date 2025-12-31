use crate::error::{AppError, Result};
use crate::models::server_metadata::{
    AgentConnection, CreateAgentConnectionRequest, UpdateAgentConnectionRequest,
};
use sqlx::postgres::PgPool;
use sqlx::Row;

pub struct AgentConnectionService {
    pool: PgPool,
}

impl AgentConnectionService {
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    pub async fn create_connection(
        &self,
        req: CreateAgentConnectionRequest,
    ) -> Result<AgentConnection> {
        let row = sqlx::query(
            r#"INSERT INTO agent_connections (node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
            ON CONFLICT (node_id) DO UPDATE SET ssh_hostname = EXCLUDED.ssh_hostname, ssh_port = EXCLUDED.ssh_port, ssh_username = EXCLUDED.ssh_username, config_path = EXCLUDED.config_path, agent_path = EXCLUDED.agent_path, location = EXCLUDED.location, environment = EXCLUDED.environment, tags = EXCLUDED.tags, os_type = EXCLUDED.os_type, installation_method = EXCLUDED.installation_method, job_id = EXCLUDED.job_id, notes = EXCLUDED.notes, updated_at = NOW()
            RETURNING id, node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes, created_at, updated_at, last_connected_at"#
        )
        .bind(&req.node_id).bind(&req.ssh_hostname).bind(req.ssh_port).bind(&req.ssh_username).bind(&req.config_path).bind(&req.agent_path).bind(&req.location).bind(&req.environment).bind(&req.tags).bind(&req.os_type).bind(&req.installation_method).bind(&req.job_id).bind(&req.notes)
        .fetch_one(&self.pool).await.map_err(AppError::Database)?;
        self.row_to_connection(&row)
    }

    pub async fn connection(&self, node_id: &str) -> Result<Option<AgentConnection>> {
        sqlx::query(
            r#"SELECT id, node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes, created_at, updated_at, last_connected_at FROM agent_connections WHERE node_id = $1"#
        ).bind(node_id).fetch_optional(&self.pool).await.map_err(AppError::Database)?
        .map(|r| self.row_to_connection(&r)).transpose()
    }

    pub async fn list_connections(&self) -> Result<Vec<AgentConnection>> {
        let rows = sqlx::query(
            r#"SELECT id, node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes, created_at, updated_at, last_connected_at FROM agent_connections ORDER BY node_id"#
        ).fetch_all(&self.pool).await.map_err(AppError::Database)?;
        rows.iter().map(|r| self.row_to_connection(r)).collect()
    }

    pub async fn update_connection(
        &self,
        node_id: &str,
        req: UpdateAgentConnectionRequest,
    ) -> Result<Option<AgentConnection>> {
        let row = sqlx::query(
            r#"UPDATE agent_connections SET ssh_hostname = COALESCE($2, ssh_hostname), ssh_port = COALESCE($3, ssh_port), ssh_username = COALESCE($4, ssh_username), config_path = COALESCE($5, config_path), agent_path = COALESCE($6, agent_path), location = COALESCE($7, location), environment = COALESCE($8, environment), tags = COALESCE($9, tags), os_type = COALESCE($10, os_type), installation_method = COALESCE($11, installation_method), notes = COALESCE($12, notes), updated_at = NOW()
            WHERE node_id = $1
            RETURNING id, node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes, created_at, updated_at, last_connected_at"#
        )
        .bind(node_id).bind(&req.ssh_hostname).bind(req.ssh_port).bind(&req.ssh_username).bind(&req.config_path).bind(&req.agent_path).bind(&req.location).bind(&req.environment).bind(&req.tags).bind(&req.os_type).bind(&req.installation_method).bind(&req.notes)
        .fetch_optional(&self.pool).await.map_err(AppError::Database)?;
        row.map(|r| self.row_to_connection(&r)).transpose()
    }

    pub async fn delete_connection(&self, node_id: &str) -> Result<bool> {
        let result = sqlx::query("DELETE FROM agent_connections WHERE node_id = $1")
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(result.rows_affected() > 0)
    }

    pub async fn update_last_connected(&self, node_id: &str) -> Result<()> {
        sqlx::query("UPDATE agent_connections SET last_connected_at = NOW() WHERE node_id = $1")
            .bind(node_id)
            .execute(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(())
    }

    pub async fn find_orphaned_agents(
        &self,
        active_ids: &[String],
    ) -> Result<Vec<AgentConnection>> {
        if active_ids.is_empty() {
            return self.list_connections().await;
        }
        let rows = sqlx::query(
            r#"SELECT id, node_id, ssh_hostname, ssh_port, ssh_username, config_path, agent_path, location, environment, tags, os_type, installation_method, job_id, notes, created_at, updated_at, last_connected_at FROM agent_connections WHERE node_id != ALL($1) ORDER BY node_id"#
        ).bind(active_ids).fetch_all(&self.pool).await.map_err(AppError::Database)?;
        rows.iter().map(|r| self.row_to_connection(r)).collect()
    }

    pub async fn connection_count(&self) -> Result<i64> {
        let row: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM agent_connections")
            .fetch_one(&self.pool)
            .await
            .map_err(AppError::Database)?;
        Ok(row.0)
    }

    fn row_to_connection(&self, r: &sqlx::postgres::PgRow) -> Result<AgentConnection> {
        Ok(AgentConnection {
            id: r.get("id"),
            node_id: r.get("node_id"),
            ssh_hostname: r.get("ssh_hostname"),
            ssh_port: r.get("ssh_port"),
            ssh_username: r.get("ssh_username"),
            config_path: r.get("config_path"),
            agent_path: r.get("agent_path"),
            location: r.get("location"),
            environment: r
                .get::<Option<String>, _>("environment")
                .unwrap_or_default(),
            tags: r.get("tags"),
            os_type: r.get("os_type"),
            installation_method: r
                .get::<Option<String>, _>("installation_method")
                .unwrap_or_default(),
            job_id: r.get("job_id"),
            notes: r.get("notes"),
            created_at: r.get("created_at"),
            updated_at: r.get("updated_at"),
            last_connected_at: r.get("last_connected_at"),
        })
    }
}
