use crate::config::CONFIG;
use crate::error::{AppError, Result};
use redis::aio::ConnectionManager;
use redis::AsyncCommands;
use serde::{de::DeserializeOwned, Serialize};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;
use tracing::info;

pub struct CacheService {
    conn: Arc<RwLock<Option<ConnectionManager>>>,
    ttl: Duration,
}

impl CacheService {
    pub fn new() -> Self {
        Self { conn: Arc::new(RwLock::new(None)), ttl: Duration::from_secs(CONFIG.cache_ttl) }
    }

    pub async fn connect(&self) -> Result<()> {
        let c = redis::Client::open(CONFIG.redis_url.as_str()).map_err(AppError::Redis)?;
        let m = ConnectionManager::new(c).await.map_err(AppError::Redis)?;
        *self.conn.write().await = Some(m);
        info!("redis connected");
        Ok(())
    }

    async fn mgr(&self) -> Result<ConnectionManager> {
        self.conn.read().await.clone()
            .ok_or_else(|| AppError::ServiceUnavailable("redis not connected".into()))
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let mut m = self.mgr().await?;
        let _: String = redis::cmd("PING").query_async(&mut m).await.map_err(AppError::Redis)?;
        Ok(true)
    }

    pub async fn get<T: DeserializeOwned>(&self, k: &str) -> Result<Option<T>> {
        let Ok(mut m) = self.mgr().await else { return Ok(None) };
        match m.get::<_, Option<String>>(k).await {
            Ok(Some(d)) => serde_json::from_str(&d).map(Some).map_err(AppError::Serialization),
            _ => Ok(None),
        }
    }

    pub async fn set<T: Serialize>(&self, k: &str, v: &T) -> Result<()> {
        self.set_with_ttl(k, v, self.ttl).await
    }

    pub async fn set_with_ttl<T: Serialize>(&self, k: &str, v: &T, ttl: Duration) -> Result<()> {
        let mut m = self.mgr().await?;
        let d = serde_json::to_string(v)?;
        m.set_ex(k, &d, ttl.as_secs()).await.map_err(AppError::Redis)
    }

    pub async fn get_raw(&self, k: &str) -> Result<Option<String>> {
        let Ok(mut m) = self.mgr().await else { return Ok(None) };
        Ok(m.get(k).await.ok().flatten())
    }

    pub async fn set_raw(&self, k: &str, v: &str, ttl: u64) -> Result<()> {
        let mut m = self.mgr().await?;
        m.set_ex(k, v, ttl).await.map_err(AppError::Redis)
    }

    pub async fn delete(&self, k: &str) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<()> = m.del(k).await; }
        Ok(())
    }

    pub async fn keys(&self, pat: &str) -> Result<Vec<String>> {
        let Ok(mut m) = self.mgr().await else { return Ok(vec![]) };
        Ok(m.keys(pat).await.unwrap_or_default())
    }

    pub async fn delete_pattern(&self, pat: &str) -> Result<usize> {
        let Ok(mut m) = self.mgr().await else { return Ok(0) };
        let ks: Vec<String> = m.keys(pat).await.unwrap_or_default();
        let n = ks.len();
        for k in ks { let _: redis::RedisResult<()> = m.del(&k).await; }
        Ok(n)
    }

    pub async fn hset<T: Serialize>(&self, k: &str, f: &str, v: &T) -> Result<()> {
        if let Ok(mut m) = self.mgr().await {
            let d = serde_json::to_string(v)?;
            let _: redis::RedisResult<()> = m.hset(k, f, &d).await;
        }
        Ok(())
    }

    pub async fn hget<T: DeserializeOwned>(&self, k: &str, f: &str) -> Result<Option<T>> {
        let Ok(mut m) = self.mgr().await else { return Ok(None) };
        match m.hget::<_, _, Option<String>>(k, f).await {
            Ok(Some(d)) => serde_json::from_str(&d).map(Some).map_err(AppError::Serialization),
            _ => Ok(None),
        }
    }

    pub async fn hgetall<T: DeserializeOwned>(&self, k: &str) -> Result<Vec<(String, T)>> {
        let Ok(mut m) = self.mgr().await else { return Ok(vec![]) };
        let pairs: Vec<(String, String)> = m.hgetall(k).await.unwrap_or_default();
        Ok(pairs.into_iter().filter_map(|(f, d)| serde_json::from_str(&d).ok().map(|v| (f, v))).collect())
    }

    pub async fn hdel(&self, k: &str, f: &str) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<()> = m.hdel(k, f).await; }
        Ok(())
    }

    pub async fn exists(&self, k: &str) -> Result<bool> {
        let Ok(mut m) = self.mgr().await else { return Ok(false) };
        Ok(m.exists(k).await.unwrap_or(false))
    }

    pub async fn expire(&self, k: &str, ttl: Duration) -> Result<()> {
        if let Ok(mut m) = self.mgr().await {
            let _: redis::RedisResult<()> = m.expire(k, ttl.as_secs() as i64).await;
        }
        Ok(())
    }

    pub async fn incr(&self, k: &str) -> Result<i64> {
        let Ok(mut m) = self.mgr().await else { return Ok(0) };
        Ok(m.incr(k, 1i64).await.unwrap_or(0))
    }

    pub async fn publish(&self, ch: &str, msg: &str) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<()> = m.publish(ch, msg).await; }
        Ok(())
    }

    pub async fn set_ex<T: Serialize>(&self, k: &str, v: &T, secs: u64) -> Result<()> {
        self.set_with_ttl(k, v, Duration::from_secs(secs)).await
    }

    pub async fn lpush(&self, k: &str, v: &str) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<i64> = m.lpush(k, v).await; }
        Ok(())
    }

    pub async fn ltrim(&self, k: &str, start: isize, stop: isize) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<()> = m.ltrim(k, start, stop).await; }
        Ok(())
    }

    pub async fn lrange(&self, k: &str, start: isize, stop: isize) -> Result<Vec<String>> {
        let Ok(mut m) = self.mgr().await else { return Ok(vec![]) };
        Ok(m.lrange(k, start, stop).await.unwrap_or_default())
    }

    pub async fn sadd(&self, k: &str, member: &str) -> Result<()> {
        if let Ok(mut m) = self.mgr().await { let _: redis::RedisResult<i64> = m.sadd(k, member).await; }
        Ok(())
    }

    pub async fn smembers(&self, k: &str) -> Result<Vec<String>> {
        let Ok(mut m) = self.mgr().await else { return Ok(vec![]) };
        Ok(m.smembers(k).await.unwrap_or_default())
    }

    pub async fn stats(&self) -> Result<CacheStats> {
        let Ok(mut m) = self.mgr().await else { return Ok(CacheStats::default()) };
        let info: String = redis::cmd("INFO").query_async(&mut m).await.unwrap_or_default();
        let mut s = CacheStats::default();
        for ln in info.lines() {
            if let Some((k, v)) = ln.split_once(':') {
                match k {
                    "used_memory" => s.used_memory = v.parse().unwrap_or(0),
                    "connected_clients" => s.connected_clients = v.parse().unwrap_or(0),
                    "total_commands_processed" => s.total_commands = v.parse().unwrap_or(0),
                    "keyspace_hits" => s.hits = v.parse().unwrap_or(0),
                    "keyspace_misses" => s.misses = v.parse().unwrap_or(0),
                    _ => {}
                }
            }
        }
        let tot = s.hits + s.misses;
        s.hit_rate = if tot > 0 { (s.hits as f64 / tot as f64) * 100.0 } else { 0.0 };
        Ok(s)
    }
}

#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct CacheStats {
    pub used_memory: u64,
    pub connected_clients: u32,
    pub total_commands: u64,
    pub hits: u64,
    pub misses: u64,
    pub hit_rate: f64,
}
