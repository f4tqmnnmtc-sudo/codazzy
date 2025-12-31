
export type DeviceStatus = 'discovered' | 'configured' | 'running' | 'stopped' | 'ignored';
export type ScanStatus = 'pending' | 'running' | 'completed' | 'failed' | 'cancelled';
export type ProtocolType = 'snmp' | 'ssh' | 'http' | 'https' | 'mqtt' | 'telnet';


export interface DiscoveredDevice {
  id: string;
  ip_address: string;
  mac_address?: string;
  hostname?: string;
  status: string;
  device_type: string;
  vendor?: string;
  os?: string;
  discovered_at: string;
  last_seen: string;
  open_ports: number[];
  available_protocols: string[];
  scan_id: string;
  response_time_ms: number;
  description?: string;
  // Container fields (optional)
  container_id?: string;
  container_name?: string;
  container_image?: string;
  container_status?: string;
  // Metadata
  source?: string;
  location?: string;
  environment?: string;
  tags?: string[];
  notes?: string;
}


export interface ScanConfiguration {
  target_ranges: string[];
}

export interface ScanProgress {
  percentage: number;
  current_ip: string;
  ips_scanned: number;
  total_ips: number;
  elapsed_seconds: number;
  estimated_remaining_seconds: number;
}

export interface ScanResults {
  devices_found: number;
  protocols_detected: Record<string, number>;
  errors: string[];
}

export interface ScanStatusResponse {
  scan_id: string;
  status: ScanStatus;
  progress: ScanProgress;
  results: ScanResults;
  current_phase: string;
  started_at: string;
  updated_at: string;
  completed_at?: string;
  error?: string;
}


export interface TopologyNode {
  id: string;
  ip_address: string;
  hostname?: string;
  device_type: string;
  status: string;
  label?: string;
  position?: { x: number; y: number };
}

export interface TopologyEdge {
  id: string;
  source: string;
  target: string;
  source_id?: string;
  target_id?: string;
  type?: string;
  connection_type?: string;
  animated?: boolean;
  data?: {
    detected_via?: string;
    bandwidth_mbps?: number;
  };
}

export interface NetworkTopology {
  scan_id: string;
  nodes: TopologyNode[];
  edges: TopologyEdge[];
  statistics: {
    total_nodes: number;
    total_edges: number;
    device_type_counts: Record<string, number>;
  };
  generated_at: string;
}

export type LayoutAlgorithm = 'force' | 'hierarchical' | 'radial';


export interface DeviceListResponse {
  success: boolean;
  devices: DiscoveredDevice[];
  summary: {
    total: number;
    by_status: Record<string, number>;
    by_type: Record<string, number>;
  };
}


export interface DeviceConfigurationRequest {
  configuration_type: string;
  metadata?: Record<string, unknown>;
}

export interface DeviceConfigurationResponse {
  device_id: string;
  configuration_status: string;
  monitoring_type: string;
  new_id: string;
}

export interface ConnectionTestRequest {
  protocol: ProtocolType;
  connection: {
    host: string;
    port: number;
    credentials?: Record<string, string>;
  };
  tests?: ('connectivity' | 'authentication' | 'data_retrieval')[];
}

export interface ConnectionTestResponse {
  device_id: string;
  test_results: Record<string, { status: string; response_time_ms?: number }>;
  overall_status: string;
  recommendations?: string[];
}
