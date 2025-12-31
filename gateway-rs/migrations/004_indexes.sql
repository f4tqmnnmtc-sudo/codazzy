-- Índices para optimización de queries

CREATE INDEX IF NOT EXISTS idx_agent_connections_node_id ON agent_connections(node_id);
CREATE INDEX IF NOT EXISTS idx_detected_services_node_id ON detected_services(node_id);
CREATE INDEX IF NOT EXISTS idx_detected_processes_node_id ON detected_processes(node_id);
CREATE INDEX IF NOT EXISTS idx_alerts_node_id ON alerts(node_id);
CREATE INDEX IF NOT EXISTS idx_alerts_status ON alerts(status);
CREATE INDEX IF NOT EXISTS idx_alerts_created_at ON alerts(created_at);
CREATE INDEX IF NOT EXISTS idx_discovered_devices_ip ON discovered_devices(ip_address);
CREATE INDEX IF NOT EXISTS idx_discovered_devices_type ON discovered_devices(device_type);
CREATE INDEX IF NOT EXISTS idx_discovered_devices_source ON discovered_devices(source);
CREATE INDEX IF NOT EXISTS idx_discovered_devices_status ON discovered_devices(status);
CREATE INDEX IF NOT EXISTS idx_server_documents_node_id ON server_documents(node_id);
CREATE INDEX IF NOT EXISTS idx_server_processes_device_id ON server_processes(device_id);
CREATE INDEX IF NOT EXISTS idx_server_processes_collected_at ON server_processes(collected_at);
CREATE INDEX IF NOT EXISTS idx_server_services_device_id ON server_services(device_id);
CREATE INDEX IF NOT EXISTS idx_device_thresholds_device_id ON device_thresholds(device_id);

