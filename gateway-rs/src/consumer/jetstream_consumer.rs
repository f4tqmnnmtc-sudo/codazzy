use std::sync::{atomic::{AtomicU64, Ordering::Relaxed}, Arc};
use std::time::Duration;
use async_nats::jetstream::{self, consumer::PullConsumer, stream::Stream, Message};
use futures_util::StreamExt;
use parking_lot::RwLock;
use tokio::sync::Semaphore;
use tracing::{error, info, warn};

use crate::{config::CONFIG, consumer::message_handler::MessageHandler, error::{AppError, Result}, processing::InfluxWriter};

#[derive(Debug, Clone, Default)]
pub struct ConsumerStats {
    pub recv: u64, pub ok: u64, pub err: u64, pub bytes: u64,
    pub acks: u64, pub naks: u64, pub workers: u64, pub peak: u64,
}

#[derive(Default)]
struct Cnt {
    recv: AtomicU64, ok: AtomicU64, err: AtomicU64, bytes: AtomicU64,
    acks: AtomicU64, naks: AtomicU64, active: AtomicU64, peak: AtomicU64,
}

pub struct JetStreamConsumer {
    nats: Option<async_nats::Client>,
    js: Option<jetstream::Context>,
    stream: Option<Stream>,
    consumer: Option<PullConsumer>,
    handler: Arc<MessageHandler>,
    running: Arc<RwLock<bool>>,
    cnt: Arc<Cnt>,
    sem: Arc<Semaphore>,
    workers: usize,
}

impl JetStreamConsumer {
    pub fn new(w: Arc<InfluxWriter>) -> Self {
        let n = CONFIG.consumer_workers;
        Self {
            nats: None, js: None, stream: None, consumer: None,
            handler: Arc::new(MessageHandler::new(w)),
            running: Arc::new(RwLock::new(false)),
            cnt: Arc::new(Cnt::default()),
            sem: Arc::new(Semaphore::new(n)),
            workers: n,
        }
    }

    pub fn with_processes_service(w: Arc<InfluxWriter>, procs: Arc<crate::services::ServerProcessesService>) -> Self {
        let n = CONFIG.consumer_workers;
        Self {
            nats: None, js: None, stream: None, consumer: None,
            handler: Arc::new(MessageHandler::with_processes_service(w, procs)),
            running: Arc::new(RwLock::new(false)),
            cnt: Arc::new(Cnt::default()),
            sem: Arc::new(Semaphore::new(n)),
            workers: n,
        }
    }

    pub async fn connect(&mut self) -> Result<()> {
        let client = async_nats::connect(&CONFIG.nats_url).await
            .map_err(|e| AppError::Nats(e.to_string()))?;
        let js = jetstream::new(client.clone());
        let stream = self.get_or_create_stream(&js).await?;
        let consumer = self.get_or_create_consumer(&stream).await?;
        self.nats = Some(client);
        self.js = Some(js);
        self.stream = Some(stream);
        self.consumer = Some(consumer);
        Ok(())
    }

    async fn get_or_create_stream(&self, js: &jetstream::Context) -> Result<Stream> {
        let name = &CONFIG.stream_name;
        if let Ok(s) = js.get_stream(name).await { return Ok(s); }

        js.create_stream(jetstream::stream::Config {
            name: name.clone(),
            subjects: vec!["metrics.>".into(), "SystemMetrics".into()],
            retention: jetstream::stream::RetentionPolicy::Limits,
            max_messages: 10_000_000,
            max_bytes: 5 << 30,
            max_age: Duration::from_secs(CONFIG.nats_stream_max_age_days * 86400),
            storage: jetstream::stream::StorageType::File,
            num_replicas: 1,
            duplicate_window: Duration::from_secs(CONFIG.nats_duplicate_window_secs),
            ..Default::default()
        }).await.map_err(|e| AppError::Nats(e.to_string()))
    }

    async fn get_or_create_consumer(&self, stream: &Stream) -> Result<PullConsumer> {
        let name = &CONFIG.consumer_name;
        if let Ok(c) = stream.get_consumer(name).await { return Ok(c); }

        stream.create_consumer(jetstream::consumer::pull::Config {
            durable_name: Some(name.clone()),
            name: Some(name.clone()),
            ack_policy: jetstream::consumer::AckPolicy::Explicit,
            ack_wait: Duration::from_secs(CONFIG.nats_ack_wait_secs),
            max_deliver: 3,
            filter_subjects: vec!["metrics.>".into(), "SystemMetrics".into()],
            max_ack_pending: (self.workers * 2) as i64,
            ..Default::default()
        }).await.map_err(|e| AppError::Nats(e.to_string()))
    }

    pub async fn start(&self) -> Result<()> {
        let consumer = self.consumer.as_ref()
            .ok_or_else(|| AppError::Nats("not connected".into()))?;
        *self.running.write() = true;

        let mut msgs = consumer.messages().await
            .map_err(|e| AppError::NatsSubscription(e.to_string()))?;

        while let Some(res) = msgs.next().await {
            if !*self.running.read() { break }
            match res {
                Ok(msg) => {
                    self.cnt.recv.fetch_add(1, Relaxed);
                    self.cnt.bytes.fetch_add(msg.payload.len() as u64, Relaxed);
                    self.dispatch(msg).await;
                }
                Err(e) => error!("msg recv: {e}"),
            }
        }

        let _ = self.sem.acquire_many(self.workers as u32).await;
        info!("consumer stopped - {:?}", self.stats());
        Ok(())
    }

    async fn dispatch(&self, msg: Message) {
        let Ok(permit) = self.sem.clone().acquire_owned().await else { return };

        let active = self.cnt.active.fetch_add(1, Relaxed) + 1;
        self.cnt.peak.fetch_max(active, Relaxed);

        let (handler, cnt, payload) = (self.handler.clone(), self.cnt.clone(), msg.payload.to_vec());
        tokio::spawn(async move {
            match handler.handle_message(&payload).await {
                Ok(_) => {
                    if let Err(e) = msg.ack().await { warn!("ack: {e}"); }
                    else { cnt.acks.fetch_add(1, Relaxed); cnt.ok.fetch_add(1, Relaxed); }
                }
                Err(_) => {
                    let _ = msg.ack_with(async_nats::jetstream::AckKind::Nak(None)).await;
                    cnt.naks.fetch_add(1, Relaxed);
                    cnt.err.fetch_add(1, Relaxed);
                }
            }
            cnt.active.fetch_sub(1, Relaxed);
            drop(permit);
        });
    }

    pub fn stop(&self) { *self.running.write() = false; }

    pub fn stats(&self) -> ConsumerStats {
        ConsumerStats {
            recv: self.cnt.recv.load(Relaxed),
            ok: self.cnt.ok.load(Relaxed),
            err: self.cnt.err.load(Relaxed),
            bytes: self.cnt.bytes.load(Relaxed),
            acks: self.cnt.acks.load(Relaxed),
            naks: self.cnt.naks.load(Relaxed),
            workers: self.cnt.active.load(Relaxed),
            peak: self.cnt.peak.load(Relaxed),
        }
    }

    #[inline] pub fn running(&self) -> bool { *self.running.read() }
    #[inline] pub fn worker_count(&self) -> usize { self.workers }
    #[inline] pub fn active_workers(&self) -> u64 { self.cnt.active.load(Relaxed) }
}
