use crate::error::{AppError, Result};
use crate::models::teleco_device::ConnectionConfig;
use rumqttc::{AsyncClient, Event, MqttOptions, Packet, QoS};
use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::RwLock;

pub struct MqttCollector {
    host: String,
    port: u16,
    client_id: String,
    username: Option<String>,
    password: Option<String>,
    topics: Vec<String>,
    metrics: Arc<RwLock<HashMap<String, f64>>>,
}

impl MqttCollector {
    pub fn new(config: &ConnectionConfig) -> Self {
        let client_id = config
            .credentials
            .get("client_id")
            .cloned()
            .unwrap_or_else(|| format!("codazzy-collector-{}", uuid::Uuid::new_v4()));

        let topics = config
            .additional_params
            .get("topics")
            .and_then(|v| v.as_array())
            .map(|arr| {
                arr.iter()
                    .filter_map(|v| v.as_str().map(String::from))
                    .collect()
            })
            .unwrap_or_else(|| vec!["metrics/#".to_string()]);

        Self {
            host: config.host.clone(),
            port: config.port,
            client_id,
            username: config.credentials.get("username").cloned(),
            password: config.credentials.get("password").cloned(),
            topics,
            metrics: Arc::new(RwLock::new(HashMap::new())),
        }
    }

    fn build_options(&self, keep_alive_secs: u64) -> MqttOptions {
        let mut options = MqttOptions::new(&self.client_id, &self.host, self.port);
        options.set_keep_alive(Duration::from_secs(keep_alive_secs));
        if let (Some(user), Some(pass)) = (&self.username, &self.password) {
            options.set_credentials(user, pass);
        }
        options
    }

    pub async fn test_connection(&self) -> Result<bool> {
        let (client, mut eventloop) = AsyncClient::new(self.build_options(5), 10);

        match tokio::time::timeout(Duration::from_secs(5), eventloop.poll()).await {
            Ok(Ok(Event::Incoming(Packet::ConnAck(_)))) | Ok(Ok(_)) => {
                let _ = client.disconnect().await;
                Ok(true)
            }
            _ => Ok(false),
        }
    }

    pub async fn start_collection(&self) -> Result<()> {
        let (client, mut eventloop) = AsyncClient::new(self.build_options(30), 100);

        for topic in &self.topics {
            client
                .subscribe(topic, QoS::AtLeastOnce)
                .await
                .map_err(|e| AppError::Mqtt(format!("Error suscribiendo: {}", e)))?;
        }

        let metrics = self.metrics.clone();

        tokio::spawn(async move {
            loop {
                match eventloop.poll().await {
                    Ok(Event::Incoming(Packet::Publish(publish))) => {
                        let topic = publish.topic.clone();
                        let payload = String::from_utf8_lossy(&publish.payload);

                        if let Ok(value) = payload.parse::<f64>() {
                            metrics.write().await.insert(topic, value);
                        } else if let Ok(json) = serde_json::from_str::<serde_json::Value>(&payload)
                        {
                            if let Some(obj) = json.as_object() {
                                let mut m = metrics.write().await;
                                for (key, val) in obj {
                                    if let Some(num) = val.as_f64() {
                                        m.insert(format!("{}_{}", topic, key), num);
                                    }
                                }
                            }
                        }
                    }
                    Ok(Event::Incoming(Packet::Disconnect)) | Err(_) => break,
                    _ => {}
                }
            }
        });

        Ok(())
    }

    pub async fn get_metrics(&self) -> HashMap<String, f64> {
        self.metrics.read().await.clone()
    }

    pub async fn publish(&self, topic: &str, payload: &str) -> Result<()> {
        let (client, mut eventloop) = AsyncClient::new(self.build_options(5), 10);

        loop {
            match eventloop.poll().await {
                Ok(Event::Incoming(Packet::ConnAck(_))) => break,
                Ok(_) => continue,
                Err(e) => return Err(AppError::Mqtt(format!("Conexion fallida: {}", e))),
            }
        }

        client
            .publish(topic, QoS::AtLeastOnce, false, payload.as_bytes())
            .await
            .map_err(|e| AppError::Mqtt(format!("Publicacion fallida: {}", e)))?;

        Ok(())
    }
}
