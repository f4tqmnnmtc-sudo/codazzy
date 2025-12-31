use thiserror::Error;

use crate::types::Ms;

#[derive(Error, Debug)]
pub enum AgentError {
    #[error("config: {0}")]
    ConfigError(String),
    
    #[error("io: {0}")]
    IoError(#[from] std::io::Error),
    
    #[error("timeout después de {0}ms")]
    Timeout(Ms),
    
    #[error("transporte: {0}")]
    TransportError(String),
    
    #[error("serialización: {0}")]
    SerializationError(String),
    
    #[error("collector '{0}' timeout")]
    CollectorTimeout(String),
}

impl AgentError {
    pub fn config(msg: impl Into<String>) -> Self {
        Self::ConfigError(msg.into())
    }
    
    pub fn collection_timeout(ms: Ms) -> Self {
        Self::Timeout(ms)
    }
    
    #[allow(dead_code)]
    pub fn collector_timeout(name: impl Into<String>) -> Self {
        Self::CollectorTimeout(name.into())
    }
    
    pub fn transport(msg: impl Into<String>) -> Self {
        Self::TransportError(msg.into())
    }
    
    #[allow(dead_code)]
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::SerializationError(msg.into())
    }
}
