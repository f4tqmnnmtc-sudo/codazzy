use sysinfo::Networks;
use tracing::debug;

use crate::config::Config;
use crate::error::AgentError;
use crate::metrics::{NetworkInterface, NetworkMetrics};

pub struct NetworkCollector {
    // TODO FIX inevstigar
    #[allow(dead_code)]
    config_snapshot: NetworkConfigSnapshot,
}

#[derive(Clone, Copy)]
struct NetworkConfigSnapshot {
    #[allow(dead_code)]
    latency_monitoring: bool,
    #[allow(dead_code)]
    dns_metrics: bool,
}

impl NetworkCollector {
    pub fn new(cfg: &Config) -> Result<Self, AgentError> {
        let snapshot = NetworkConfigSnapshot {
            latency_monitoring: cfg.collection.network.latency_monitoring,
            dns_metrics: cfg.collection.network.dns_metrics,
        };
        
        Ok(Self { config_snapshot: snapshot })
    }

    pub async fn collect(&mut self) -> Result<NetworkMetrics, AgentError> {
        let networks = Networks::new_with_refreshed_list();
        
        let interfaces = networks.iter()
            .map(|(name, data)| {
                let rx_bytes = data.total_received();
                let tx_bytes = data.total_transmitted();
                
                debug!(
                    "interfaz {}: rx={} tx={} pkts_rx={} pkts_tx={}",
                    name, rx_bytes, tx_bytes,
                    data.total_packets_received(),
                    data.total_packets_transmitted()
                );
                
                NetworkInterface {
                    name: name.to_owned(),
                    bytes_sent: tx_bytes,
                    bytes_received: rx_bytes,
                    packets_sent: data.total_packets_transmitted(),
                    packets_received: data.total_packets_received(),
                    errors_in: data.total_errors_on_received(),
                    errors_out: data.total_errors_on_transmitted(),
                    is_up: rx_bytes > 0 || tx_bytes > 0, // TODO pensar otra manera, está corriendo si es mayor que 0
                }
            })
            .collect();
        
        Ok(NetworkMetrics { interfaces })
    }
}
