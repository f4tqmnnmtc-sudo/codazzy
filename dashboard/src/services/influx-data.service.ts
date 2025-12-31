import { ChronosDataPoint, ChronosMetrics } from "./chronos.service";
import { getApiBaseUrl } from "@/lib/api-config";

export interface InfluxMetricPoint {
  timestamp: string;
  value: number;
  server_id?: string;
  metric_type: string;
  unit?: string;
  tags?: Record<string, string>;
}

export interface ServerInfo {
  server_id: string;
  server_name: string;
  server_type: "agent" | "remote";
  status: "online" | "offline" | "error";
  last_seen: string;
  location?: string;
  protocol?: string;
  device_type?: string;
  available_metrics?: string[];
}

export interface MetricType {
  metric_name: string;
  metric_type: string;
  unit: string;
  description: string;
  available_servers: string[];
  category?: "agent" | "network" | "iot" | "other";
  source?: "agent" | "remote";
}

const COMPONENT_MAP: Record<string, string> = {
  cpu: "cpu",
  memory: "memory_percent",
  memory_used: "memory_used",
  memory_percent: "memory_percent",
  disk: "fs__usage_percent",  // "/" se convierte a "_" → "fs__usage_percent"
  disk_used: "fs__used_space",
  disk_percent: "fs__usage_percent",
  load_1: "load_1",
  load_5: "load_5",
  load_15: "load_15",
  temperature: "temp_CPU Package",
};

// Fallback components para compatibilidad
const COMPONENT_FALLBACKS: Record<string, string[]> = {
  disk: ["fs__usage_percent", "fs_root_usage_percent", "fs_home_usage_percent"],
  memory: ["memory_percent"],
};

const UNIT_MAP: Record<string, string> = {
  cpu: "%",
  memory: "GB",
  disk: "%",
  network: "Mbps",
  temperature: "C",
};

class InfluxDataService {
  private get gatewayUrl(): string {
    return getApiBaseUrl();
  }

  async getAvailableServers(): Promise<ServerInfo[]> {
    const servers: ServerInfo[] = [];
    const remoteIds = new Set<string>();

    try {
      const telecoRes = await fetch(`${this.gatewayUrl}/api/v1/teleco/devices`);
      if (telecoRes.ok) {
        const data = await telecoRes.json();
        for (const device of data.devices || []) {
          const id = device.device_id || device.id;
          remoteIds.add(id);

          let availableMetrics: string[] = [];
          try {
            const metricsRes = await fetch(
              `${this.gatewayUrl}/api/v1/metrics/timeseries?measurement=metrics_v2&field=value&time_range=1h&node_id=${id}`
            );
            if (metricsRes.ok) {
              const metricsData = await metricsRes.json();
              const components = new Set<string>();
              (metricsData.data || []).forEach((p: { component?: string }) => {
                if (p.component) components.add(p.component);
              });
              availableMetrics = Array.from(components);
            }
          } catch {}

          if (availableMetrics.length === 0) {
            availableMetrics = device.metrics_config?.enabled_metrics || ["interface_count"];
          }

          servers.push({
            server_id: id,
            server_name: device.device_name || id,
            server_type: "remote",
            status: device.status === "online" ? "online" : device.status === "error" ? "error" : "offline",
            last_seen: device.last_seen || new Date().toISOString(),
            location: device.location || "Unknown",
            protocol: device.connection_config?.protocol?.toUpperCase() || "SNMP",
            device_type: device.device_type || "standard",
            available_metrics: availableMetrics,
          });
        }
      }
    } catch {}

    try {
      const agentsRes = await fetch(`${this.gatewayUrl}/api/v1/metrics/agents`);
      if (agentsRes.ok) {
        const data = await agentsRes.json();
        for (const agent of data.agents || []) {
          const id = agent.id || agent.name;
          if (remoteIds.has(id)) continue;

          servers.push({
            server_id: id,
            server_name: agent.name || agent.id,
            server_type: "agent",
            status: agent.status === "online" ? "online" : "offline",
            last_seen: agent.last_seen || new Date().toISOString(),
            location: agent.location || "Unknown",
            available_metrics: ["cpu", "memory", "disk", "network", "temperature"],
          });
        }
      }
    } catch {}

    return servers.sort((a, b) => {
      if (a.server_type !== b.server_type) return a.server_type === "agent" ? -1 : 1;
      return a.server_name.localeCompare(b.server_name);
    });
  }

  async getAvailableMetricTypes(): Promise<MetricType[]> {
    const servers = await this.getAvailableServers();
    const agentIds = servers.filter((s) => s.server_type === "agent").map((s) => s.server_id);
    const remoteServers = servers.filter((s) => s.server_type === "remote");

    const metrics: MetricType[] = [];

    if (agentIds.length > 0) {
      metrics.push(
        { metric_name: "CPU", metric_type: "cpu", unit: "%", description: "Uso de CPU", available_servers: agentIds },
        { metric_name: "Memoria", metric_type: "memory", unit: "%", description: "Uso de memoria", available_servers: agentIds },
        { metric_name: "Disco", metric_type: "disk", unit: "%", description: "Uso de disco", available_servers: agentIds }
      );
    }

    const iotMetrics = new Map<string, { servers: string[]; unit: string; description: string }>();
    const iotInfo: Record<string, { unit: string; description: string }> = {
      interface_1_bytes_in: { unit: "Bytes", description: "Interfaz 1 - Entrada" },
      interface_1_bytes_out: { unit: "Bytes", description: "Interfaz 1 - Salida" },
      interface_2_bytes_in: { unit: "Bytes", description: "Interfaz 2 - Entrada" },
      interface_2_bytes_out: { unit: "Bytes", description: "Interfaz 2 - Salida" },
      extend_battery_level: { unit: "%", description: "UPS - Nivel Bateria" },
      extend_battery_voltage: { unit: "V", description: "UPS - Voltaje" },
      extend_ups_load: { unit: "%", description: "UPS - Carga" },
      extend_temperature: { unit: "C", description: "Sensor - Temperatura" },
      extend_humidity: { unit: "%", description: "Sensor - Humedad" },
    };

    for (const server of remoteServers) {
      for (const metricName of server.available_metrics || []) {
        if (iotInfo[metricName]) {
          if (!iotMetrics.has(metricName)) {
            iotMetrics.set(metricName, { servers: [], ...iotInfo[metricName] });
          }
          iotMetrics.get(metricName)!.servers.push(server.server_id);
        }
      }
    }

    for (const [type, info] of iotMetrics) {
      metrics.push({
        metric_name: info.description,
        metric_type: type,
        unit: info.unit,
        description: info.description,
        available_servers: info.servers,
      });
    }

    return metrics.sort((a, b) => {
      const agentTypes = ["cpu", "memory", "disk", "network"];
      const aIsAgent = agentTypes.includes(a.metric_type);
      const bIsAgent = agentTypes.includes(b.metric_type);
      if (aIsAgent && !bIsAgent) return -1;
      if (!aIsAgent && bIsAgent) return 1;
      return a.metric_name.localeCompare(b.metric_name);
    });
  }

  async getHistoricalData(
    serverId: string,
    metricType: string,
    timeRange: string = "24h",
    limit: number = 1440
  ): Promise<InfluxMetricPoint[]> {
    const componentName = COMPONENT_MAP[metricType] || metricType;
    const fallbacks = COMPONENT_FALLBACKS[metricType] || [componentName];
    const isTraffic = componentName.includes("bytes_in") || componentName.includes("bytes_out");

    // Intentar con el componente principal y fallbacks
    let data: { data?: Array<{ time: string; value: number }> } = { data: [] };
    
    for (const comp of [componentName, ...fallbacks]) {
      try {
        const res = await fetch(
          `${this.gatewayUrl}/api/v1/metrics/timeseries?measurement=metrics_v2&field=value&component=${comp}&time_range=${timeRange}&node_id=${serverId}`
        );

        if (!res.ok) continue;

        const result = await res.json();
        if (result.data?.length > 0) {
          data = result;
          break;
        }
      } catch {
        continue;
      }
    }

    if (!data.data?.length) return [];

    const sorted = data.data.sort(
      (a: { time: string }, b: { time: string }) => new Date(a.time).getTime() - new Date(b.time).getTime()
    );

    let result: InfluxMetricPoint[];

    if (isTraffic) {
      result = [];
      for (let i = 1; i < sorted.length; i++) {
        const prev = sorted[i - 1];
        const curr = sorted[i];
        const timeDiff = (new Date(curr.time).getTime() - new Date(prev.time).getTime()) / 1000;

        if (timeDiff > 0) {
          const bytesDiff = curr.value >= prev.value ? curr.value - prev.value : curr.value;
          const mbps = (bytesDiff * 8) / (timeDiff * 1_000_000);

          result.push({
            timestamp: curr.time,
            value: Math.round(mbps * 100) / 100,
            server_id: serverId,
            metric_type: metricType,
            unit: "Mbps",
          });
        }
      }
    } else {
      result = sorted.map((point: { time: string; value: number }) => ({
        timestamp: point.time,
        value: metricType === "memory_used" ? point.value / (1024 * 1024 * 1024) : point.value,
        server_id: serverId,
        metric_type: metricType,
        unit: UNIT_MAP[metricType] || "",
      }));
    }

    return limit > 0 ? result.slice(-limit) : result;
  }

  convertToChronosFormat(data: InfluxMetricPoint[], seriesName: string, serverId?: string): ChronosMetrics {
    const dataPoints: ChronosDataPoint[] = data.map((point) => {
      let ts = point.timestamp;
      try {
        const date = new Date(ts);
        ts = isNaN(date.getTime())
          ? new Date().toISOString().replace(/\.\d{3}Z$/, "+00:00")
          : date.toISOString().replace(/\.\d{3}Z$/, "+00:00");
      } catch {
        ts = new Date().toISOString().replace(/\.\d{3}Z$/, "+00:00");
      }
      return { timestamp: ts, value: point.value };
    });

    return {
      series_name: seriesName,
      server_id: serverId,
      metric_type: data[0]?.metric_type || "cpu",
      unit: data[0]?.unit || "%",
      data_points: dataPoints,
    };
  }
}

export const influxDataService = new InfluxDataService();
