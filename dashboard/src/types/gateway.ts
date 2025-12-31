export interface Agent {
  id: string;
  name: string;
  status: string;
  cpu_usage: number;
  memory_usage: number;
  last_seen: string;
  type?: string;
  location?: string;
  network_tx_bytes?: number;
  network_rx_bytes?: number;
  network_tx_rate?: number;
  network_rx_rate?: number;
}

export interface Alert {
  id: string;
  node_id: string;
  severity: 'info' | 'warning' | 'critical';
  metric_name: string;
  value: number;
  message: string;
  created_at: string;
}

export interface ServerConnection {
  node_id: string;
  ssh_hostname: string;
  ssh_port: number;
  ssh_username: string;
  config_path: string;
  agent_path: string;
  location: string;
  environment: string;
  os_type: string;
}

export interface ServerDocument {
  id: number;
  node_id: string;
  filename: string;
  file_type: string;
  file_size: number;
  content: string;
  description?: string;
  created_at: string;
  updated_at: string;
}
