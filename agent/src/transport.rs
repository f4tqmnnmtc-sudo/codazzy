use std::collections::VecDeque;
use std::time::Duration;
use tokio::time::Instant;
use tracing::{debug, info, warn};

use crate::config::TransportConfig as ConfigTransport;
use crate::error::AgentError;
use crate::metrics::SystemMetrics;
use crate::types::Count;

#[derive(Debug, Clone)]
pub struct TransportConfig {
    pub nats_url: String,
    pub topic_prefix: String,
    pub buffer_size: usize,
    pub compression: bool,
    #[allow(dead_code)]
    pub retry_attempts: u32, // TODO: implementar reintentos con backoff
    pub batch_size: usize,
    pub flush_interval: Duration,
}

impl Default for TransportConfig {
    fn default() -> Self {
        Self {
            nats_url: "nats://localhost:4222".into(),
            topic_prefix: "metrics".into(),
            buffer_size: 1000,
            compression: true,
            retry_attempts: 3,
            batch_size: 10,
            flush_interval: Duration::from_secs(30),
        }
    }
}

impl From<&ConfigTransport> for TransportConfig {
    fn from(cfg: &ConfigTransport) -> Self {
        Self {
            nats_url: cfg.nats_url.clone(),
            topic_prefix: cfg.topic_prefix.clone(),
            buffer_size: cfg.buffer_size,
            compression: cfg.compression,
            retry_attempts: cfg.retry_attempts,
            batch_size: cfg.batch_size,
            flush_interval: Duration::from_secs(cfg.flush_interval),
        }
    }
}

impl TransportConfig {
    pub fn from_config(cfg: &ConfigTransport) -> Self {
        Self::from(cfg)
    }
}

pub struct NatsTransport {
    client: Option<async_nats::Client>,
    buffer: VecDeque<SystemMetrics>,
    config: TransportConfig,
    last_flush: Instant,
    msgs_sent: Count,
    msgs_dropped: Count,
}

impl NatsTransport {
    pub async fn new(config: TransportConfig) -> Result<Self, AgentError> {
        let buffer_cap = config.buffer_size;
        
        let mut transport = Self {
            client: None,
            buffer: VecDeque::with_capacity(buffer_cap),
            config,
            last_flush: Instant::now(),
            msgs_sent: 0,
            msgs_dropped: 0,
        };
        
        if let Err(e) = transport.try_connect().await {
            warn!("NATS no disponible al inicio: {}", e);
            warn!("buffering métricas hasta reconexión");
        }
        
        Ok(transport)
    }

    async fn try_connect(&mut self) -> Result<(), AgentError> {
        debug!("conectando a NATS: {}", self.config.nats_url);
        
        let client = async_nats::connect(&self.config.nats_url)
            .await
            .map_err(|e| AgentError::transport(format!(
                "conexión fallida a {}: {}", 
                self.config.nats_url, e
            )))?;
        
        self.client = Some(client);
        info!("conexión NATS establecida");
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.client
            .as_ref()
            .map(|c| c.connection_state() == async_nats::connection::State::Connected)
            .unwrap_or(false)
    }

    fn check_connection_health(&mut self) {
        if let Some(ref client) = self.client {
            let state = client.connection_state();
            if state != async_nats::connection::State::Connected {
                warn!("NATS caído, buffering...");
                self.client = None;
            }
        }
    }

    pub async fn send_metrics(&mut self, metrics: SystemMetrics) -> Result<(), AgentError> {
        self.check_connection_health();

        if self.client.is_none() {
            if let Err(_) = self.try_connect().await {
                self.add_to_buffer(metrics);
                return Ok(()); // No es error crítico, tenemos buffer
            }
        }

        match self.do_publish(&metrics).await {
            Ok(()) => {
                self.msgs_sent += 1;
                
                if !self.buffer.is_empty() {
                    let _ = self.flush_buffer().await;
                }
            }
            Err(_) => {
                self.add_to_buffer(metrics);
                self.client = None;
            }
        }
        
        Ok(())
    }

    async fn do_publish(&self, metrics: &SystemMetrics) -> Result<(), AgentError> {
        let client = self.client.as_ref()
            .ok_or_else(|| AgentError::transport("no NATS connection"))?;
        
        if client.connection_state() != async_nats::connection::State::Connected {
            return Err(AgentError::transport("conexión NATS no disponible"));
        }
        
        let payload = rmp_serde::to_vec(metrics)
            .map_err(|e| AgentError::SerializationError(format!("msgpack: {}", e)))?;
        
        let final_payload = if self.config.compression {
            self.compress_payload(&payload)
        } else {
            payload
        };
        
        let topic = format!("{}.{}", self.config.topic_prefix, metrics.node_id);
        
        client.publish(topic, final_payload.into())
            .await
            .map_err(|e| AgentError::transport(format!("publish: {}", e)))
    }

    fn add_to_buffer(&mut self, metrics: SystemMetrics) {
        if self.buffer.len() >= self.config.buffer_size {
            if let Some(_) = self.buffer.pop_front() {
                self.msgs_dropped += 1;
                warn!("buffer lleno, descartando métrica antigua (perdidas: {})", self.msgs_dropped);
            }
        }
        self.buffer.push_back(metrics);
        info!("buffering métrica, pendientes: {}", self.buffer.len());
    }

    pub async fn flush_buffer(&mut self) -> Result<(), AgentError> {
        if self.buffer.is_empty() {
            return Ok(());
        }
        
        self.check_connection_health();
        
        let pending = self.buffer.len();
        
        if self.client.is_none() {
            self.try_connect().await?;
        }

        let mut failed = Vec::new();
        let mut sent = 0;
        
        while let Some(metrics) = self.buffer.pop_front() {
            match self.do_publish(&metrics).await {
                Ok(()) => {
                    sent += 1;
                    self.msgs_sent += 1;
                }
                Err(_) => {
                    failed.push(metrics);
                    failed.extend(self.buffer.drain(..));
                    self.client = None;
                    break; // si falla uno, fallarán todos
                }
            }
        }
        
        for m in failed.into_iter().rev() {
            self.buffer.push_front(m);
        }
        
        self.last_flush = Instant::now();
        
        if sent > 0 {
            info!("buffer recuperado: {}/{} enviados", sent, pending);
        }
        
        Ok(())
    }

    #[inline]
    fn compress_payload(&self, data: &[u8]) -> Vec<u8> {
        lz4_flex::compress_prepend_size(data)
    }

    pub fn should_flush(&self) -> bool {
        if self.buffer.is_empty() {
            return false;
        }
        
        self.buffer.len() >= self.config.batch_size 
            || self.last_flush.elapsed() >= self.config.flush_interval
    }

    pub fn get_stats(&self) -> TransportStats {
        TransportStats {
            connected: self.is_connected(),
            buffer_size: self.buffer.len(),
            buffer_capacity: self.config.buffer_size,
            msgs_sent: self.msgs_sent,
            msgs_dropped: self.msgs_dropped,
        }
    }
}

#[derive(Debug)]
pub struct TransportStats {
    pub connected: bool,
    pub buffer_size: usize,
    pub buffer_capacity: usize,
    #[allow(dead_code)]
    msgs_sent: Count,
    #[allow(dead_code)]
    msgs_dropped: Count,
}

impl Drop for NatsTransport {
    fn drop(&mut self) {
        let pending = self.buffer.len();
        if pending > 0 {
            // TODO FIX darle una vuelta... https://tokio.rs/tokio/topics/shutdown
            warn!(
                "transporte cerrado con {} mensajes sin enviar (total perdidos: {})",
                pending,
                self.msgs_dropped + pending as Count
            );
        }
    }
}
