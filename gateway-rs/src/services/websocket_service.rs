use axum::extract::ws::{Message, WebSocket};
use dashmap::DashMap;
use futures_util::{SinkExt, StreamExt};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tokio::sync::{broadcast, mpsc};
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data")]
pub enum WsMessage {
    #[serde(rename = "subscribe")] Subscribe { topic: String },
    #[serde(rename = "unsubscribe")] Unsubscribe { topic: String },
    #[serde(rename = "pong")] Pong,
    #[serde(rename = "ping")] Ping,
    #[serde(rename = "update")] Update { topic: String, data: serde_json::Value },
    #[serde(rename = "error")] Error { message: String },
}

#[allow(dead_code)]
struct Conn { id: String, subs: HashSet<String>, tx: mpsc::Sender<String> }

pub struct WebSocketService {
    conns: DashMap<String, Conn>,
    topics: DashMap<String, HashSet<String>>,
    bcast_tx: broadcast::Sender<(String, String)>,
    msg_cnt: AtomicU64,
    running: RwLock<bool>,
}

impl WebSocketService {
    pub fn new() -> Arc<Self> {
        let (tx, _) = broadcast::channel(1024);
        Arc::new(Self {
            conns: DashMap::new(), topics: DashMap::new(), bcast_tx: tx,
            msg_cnt: AtomicU64::new(0), running: RwLock::new(false),
        })
    }

    pub fn start(self: &Arc<Self>) { *self.running.write() = true; }
    pub fn stop(&self) { *self.running.write() = false; }

    pub async fn accept(self: Arc<Self>, socket: WebSocket) {
        let cid = Uuid::new_v4().to_string();
        let (mut ws_tx, mut ws_rx) = socket.split();
        let (tx, mut rx) = mpsc::channel::<String>(256);

        self.conns.insert(cid.clone(), Conn { id: cid.clone(), subs: HashSet::new(), tx });

        let mut bcast_rx = self.bcast_tx.subscribe();
        let self_tx = self.clone();
        let self_rx = self.clone();
        let id_tx = cid.clone();
        let id_rx = cid.clone();

        let send_task = tokio::spawn(async move {
            loop {
                tokio::select! {
                    Some(msg) = rx.recv() => { if ws_tx.send(Message::Text(msg)).await.is_err() { break; } }
                    Ok((topic, msg)) = bcast_rx.recv() => {
                        if let Some(c) = self_tx.conns.get(&id_tx) {
                            if c.subs.contains(&topic) || topic == "all" {
                                if ws_tx.send(Message::Text(msg)).await.is_err() { break; }
                            }
                        }
                    }
                    else => break,
                }
            }
        });

        let recv_task = tokio::spawn(async move {
            while let Some(Ok(msg)) = ws_rx.next().await {
                match msg {
                    Message::Text(txt) => self_rx.on_message(&id_rx, &txt).await,
                    Message::Close(_) => break,
                    _ => {}
                }
            }
        });

        tokio::select! { _ = send_task => {} _ = recv_task => {} }
        self.disconnect(&cid);
    }

    async fn on_message(&self, cid: &str, txt: &str) {
        match serde_json::from_str::<WsMessage>(txt) {
            Ok(WsMessage::Subscribe { topic }) => {
                self.subscribe(cid, &topic);
                self.send(cid, &serde_json::json!({"type": "subscribed", "topic": topic}).to_string()).await;
            }
            Ok(WsMessage::Unsubscribe { topic }) => {
                self.unsubscribe(cid, &topic);
                self.send(cid, &serde_json::json!({"type": "unsubscribed", "topic": topic}).to_string()).await;
            }
            Ok(WsMessage::Ping) => { self.send(cid, &serde_json::json!({"type": "pong"}).to_string()).await; }
            _ => {}
        }
    }

    pub fn subscribe(&self, cid: &str, topic: &str) {
        if let Some(mut c) = self.conns.get_mut(cid) { c.subs.insert(topic.to_string()); }
        self.topics.entry(topic.to_string()).or_insert_with(HashSet::new).insert(cid.to_string());
    }

    pub fn unsubscribe(&self, cid: &str, topic: &str) {
        if let Some(mut c) = self.conns.get_mut(cid) { c.subs.remove(topic); }
        if let Some(mut t) = self.topics.get_mut(topic) { t.remove(cid); }
    }

    pub fn disconnect(&self, cid: &str) {
        if let Some((_, c)) = self.conns.remove(cid) {
            for topic in c.subs { if let Some(mut t) = self.topics.get_mut(&topic) { t.remove(cid); } }
        }
    }

    async fn send(&self, cid: &str, msg: &str) {
        if let Some(c) = self.conns.get(cid) { let _ = c.tx.send(msg.to_string()).await; }
    }

    pub fn broadcast_to_topic(&self, topic: &str, data: serde_json::Value) {
        let msg = serde_json::json!({"type": "update", "topic": topic, "data": data, "timestamp": chrono::Utc::now().to_rfc3339()}).to_string();
        let _ = self.bcast_tx.send((topic.to_string(), msg));
        self.msg_cnt.fetch_add(1, Ordering::Relaxed);
    }

    pub fn broadcast_to_all(&self, data: serde_json::Value) {
        let msg = serde_json::json!({"type": "update", "topic": "all", "data": data, "timestamp": chrono::Utc::now().to_rfc3339()}).to_string();
        let _ = self.bcast_tx.send(("all".to_string(), msg));
        self.msg_cnt.fetch_add(1, Ordering::Relaxed);
    }

    pub fn stats(&self) -> WsStats {
        WsStats {
            active_connections: self.conns.len(),
            topics: self.topics.iter().map(|e| (e.key().clone(), e.value().len())).collect(),
            total_messages: self.msg_cnt.load(Ordering::Relaxed),
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct WsStats { pub active_connections: usize, pub topics: HashMap<String, usize>, pub total_messages: u64 }
