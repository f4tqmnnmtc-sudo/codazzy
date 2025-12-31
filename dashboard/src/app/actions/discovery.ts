"use server";

import { getApiBaseUrl } from "@/lib/api-config";

function mapScanTypeToProtocols(scanType: string): string[] {
  switch (scanType) {
    case "icmp":
    case "arp":
      return [];
    case "tcp":
    case "udp":
      return ["ssh", "http"];
    case "full":
      return ["snmp", "ssh", "http", "mqtt"];
    default:
      return ["ssh", "snmp"];
  }
}

export async function startScan(config: {
  target: string;
  scan_type?: string;
  timeout_seconds?: number;
  port_range?: string;
}): Promise<{ success: boolean; job_id?: string; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const payload = {
      target_ranges: [config.target],
      protocols: mapScanTypeToProtocols(config.scan_type || "icmp"),
      options: {
        timeout: config.timeout_seconds || 30,
        max_threads: 50,
        port_ranges: config.port_range ? [config.port_range] : ["1-1000"],
        deep_scan: config.scan_type === "full",
        detect_os: config.scan_type === "full",
      },
    };

    const res = await fetch(`${API}/api/v1/discovery/scan/start`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(payload),
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      return { success: false, error: err.message || err.detail || "Error iniciando escaneo" };
    }

    const data = await res.json();
    return { success: true, job_id: data.scan_id || data.job_id };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function getScanStatus(jobId: string): Promise<{
  status: string;
  progress?: number;
  devices_found?: number;
  error?: string;
}> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/scan/${jobId}`, { cache: "no-store" });
    if (!res.ok) {
      return { status: "error", error: "No se pudo obtener estado" };
    }
    return res.json();
  } catch {
    return { status: "error", error: "Error de conexion" };
  }
}

export async function stopScan(jobId: string): Promise<{ success: boolean; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/scan/${jobId}`, { method: "DELETE" });
    if (!res.ok) {
      return { success: false, error: "No se pudo detener escaneo" };
    }
    return { success: true };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function getDiscoveredDevices(): Promise<{
  devices: Array<{
    id: string;
    ip_address: string;
    hostname?: string;
    mac_address?: string;
    device_type?: string;
    vendor?: string;
    status: string;
    last_seen: string;
    open_ports?: number[];
    services?: string[];
  }>;
}> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/devices`, { cache: "no-store" });
    if (!res.ok) {
      return { devices: [] };
    }
    return res.json();
  } catch {
    return { devices: [] };
  }
}

export async function getDeviceDetails(deviceId: string): Promise<{
  id: string;
  ip_address: string;
  hostname?: string;
  mac_address?: string;
  device_type?: string;
  vendor?: string;
  status: string;
  last_seen: string;
  open_ports?: number[];
  services?: string[];
} | null> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/devices/${deviceId}`, { cache: "no-store" });
    if (!res.ok) return null;
    return res.json();
  } catch {
    return null;
  }
}

export async function updateDevice(
  deviceId: string,
  data: { hostname?: string; device_type?: string }
): Promise<{ success: boolean; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/devices/${deviceId}`, {
      method: "PUT",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify(data),
    });
    if (!res.ok) {
      return { success: false, error: "No se pudo actualizar dispositivo" };
    }
    return { success: true };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function deleteDevice(deviceId: string): Promise<{ success: boolean; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/discovery/devices/${deviceId}`, { method: "DELETE" });
    if (!res.ok) {
      return { success: false, error: "No se pudo eliminar dispositivo" };
    }
    return { success: true };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function clearAllDevices(): Promise<{ success: boolean; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const devicesRes = await fetch(`${API}/api/v1/discovery/devices`, { cache: "no-store" });
    if (!devicesRes.ok) {
      return { success: false, error: "No se pudo obtener dispositivos" };
    }
    
    const data = await devicesRes.json();
    const devices = data.devices || [];
    
    const deletePromises = devices.map((device: { id: string }) =>
      fetch(`${API}/api/v1/discovery/devices/${device.id}`, { method: "DELETE" })
    );
    
    await Promise.all(deletePromises);
    return { success: true };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function getNetworkTopology(algorithm?: string): Promise<{
  nodes: Array<{
    id: string;
    label: string;
    type: string;
    ip?: string;
    status?: string;
  }>;
  edges: Array<{
    source: string;
    target: string;
    weight?: number;
  }>;
}> {
  try {
    const API = getApiBaseUrl();
    const url = algorithm
      ? `${API}/api/v1/discovery/topology?algorithm=${algorithm}`
      : `${API}/api/v1/discovery/topology`;
    const res = await fetch(url, { cache: "no-store" });
    if (!res.ok) {
      return { nodes: [], edges: [] };
    }
    return res.json();
  } catch {
    return { nodes: [], edges: [] };
  }
}

export async function exportReport(
  reportId: string,
  format: string,
  content: string
): Promise<{ success: boolean; blob?: Blob; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/reports/export/${reportId}?format=${format}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ content }),
    });

    if (!res.ok) {
      const err = await res.text();
      return { success: false, error: err || "Error exportando" };
    }

    const blob = await res.blob();
    return { success: true, blob };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}
