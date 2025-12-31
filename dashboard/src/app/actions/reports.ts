"use server";

import { getApiBaseUrl } from "@/lib/api-config";

export interface ReportConfig {
  title: string;
  type: "executive" | "technical" | "network_performance";
  servers: string[];
  timeRange: string;
  includeAnomalies: boolean;
  includePredictions: boolean;
  includeRecommendations: boolean;
  language: "es" | "en";
  format: "markdown" | "html" | "pdf";
}

export interface MetricSummary {
  serverId: string;
  serverName: string;
  metricType: string;
  count: number;
  avg: number;
  min: number;
  max: number;
  lastValue?: number;
  unit?: string;
}

interface Agent {
  id: string;
  name: string;
  status: string;
  cpu_usage: number;
  memory_usage: number;
}

interface ReportData {
  servers: { id: string; name: string; type: string; status: string }[];
  anomalies: unknown[];
  predictions: unknown[];
  metrics: MetricSummary[];
}

export async function getAvailableServers(
  initialAgents: Agent[] = []
): Promise<{ servers: string[]; agents: Agent[] }> {
  const allServers: Agent[] = [];
  const allIds: string[] = [];

  for (const a of initialAgents) {
    if (!allIds.includes(a.id)) {
      allServers.push(a);
      allIds.push(a.id);
    }
  }

  const API = getApiBaseUrl();
  try {
    const res = await fetch(`${API}/api/v1/metrics/agents`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      for (const a of data.agents || []) {
        if (!allIds.includes(a.id)) {
          allServers.push(a);
          allIds.push(a.id);
        }
      }
    }
  } catch {}

  try {
    const res = await fetch(`${API}/api/v1/teleco/devices`, { cache: "no-store" });
    if (res.ok) {
      const data = await res.json();
      for (const d of data.devices || []) {
        const id = d.device_id || d.id;
        if (!allIds.includes(id)) {
          allServers.push({
            id,
            name: d.device_name || id,
            status: d.status === "online" ? "online" : "offline",
            cpu_usage: 0,
            memory_usage: 0,
          });
          allIds.push(id);
        }
      }
    }
  } catch {}

  return { servers: allIds, agents: allServers };
}

export async function collectReportData(
  agents: Agent[],
  config: ReportConfig,
  servers: string[]
): Promise<ReportData> {
  const API = getApiBaseUrl();
  const data: ReportData = { servers: [], anomalies: [], predictions: [], metrics: [] };

  for (const serverId of servers) {
    const agent = agents.find((a) => a.id === serverId);
    if (!agent) continue;

    const isRemote =
      serverId.includes("-clab") ||
      serverId.includes("router") ||
      serverId.includes("sw") ||
      (agent.cpu_usage === 0 && agent.memory_usage === 0);

    data.servers.push({
      id: serverId,
      name: agent.name || serverId,
      type: isRemote ? "network_device" : "server",
      status: agent.status || "online",
    });

    if (!isRemote && (agent.cpu_usage > 0 || agent.memory_usage > 0)) {
      data.metrics.push({
        serverId,
        serverName: agent.name || serverId,
        metricType: "cpu",
        count: 1,
        avg: agent.cpu_usage || 0,
        min: (agent.cpu_usage || 0) * 0.8,
        max: (agent.cpu_usage || 0) * 1.2,
        lastValue: agent.cpu_usage || 0,
      });

      data.metrics.push({
        serverId,
        serverName: agent.name || serverId,
        metricType: "memory",
        count: 1,
        avg: agent.memory_usage || 0,
        min: (agent.memory_usage || 0) * 0.9,
        max: (agent.memory_usage || 0) * 1.1,
        lastValue: agent.memory_usage || 0,
      });
    }

    if (isRemote || config.type === "network_performance") {
      try {
        const res = await fetch(
          `${API}/api/v1/metrics/timeseries?measurement=metrics_v2&field=value&time_range=${config.timeRange}&node_id=${serverId}`,
          { cache: "no-store" }
        );
        if (res.ok) {
          const netData = await res.json();
          const points = netData.data || [];

          const byComponent: Record<string, number[]> = {};
          for (const p of points) {
            const comp = p.component || "";
            if (!byComponent[comp]) byComponent[comp] = [];
            byComponent[comp].push(p.value);
          }

          for (const [component, values] of Object.entries(byComponent)) {
            if (component.includes("bytes_in") || component.includes("bytes_out")) {
              if (values.length >= 2) {
                const diffs: number[] = [];
                for (let i = 1; i < values.length; i++) {
                  const diff = values[i] - values[i - 1];
                  if (diff >= 0) diffs.push(diff);
                }

                if (diffs.length > 0) {
                  const avg = diffs.reduce((a, b) => a + b, 0) / diffs.length;
                  const max = Math.max(...diffs);
                  const avgMbps = (avg * 8) / (10 * 1_000_000);
                  const maxMbps = (max * 8) / (10 * 1_000_000);

                  data.metrics.push({
                    serverId,
                    serverName: agent.name || serverId,
                    metricType: component,
                    count: diffs.length,
                    avg: Math.round(avgMbps * 100) / 100,
                    min: 0,
                    max: Math.round(maxMbps * 100) / 100,
                    lastValue: Math.round(avgMbps * 100) / 100,
                    unit: "Mbps",
                  });
                }
              }
            }
          }
        }
      } catch {}
    }

    if (config.includePredictions) {
      try {
        const res = await fetch(`${API}/api/v1/predictions/${serverId}?range=7d`, { cache: "no-store" });
        if (res.ok) {
          const predData = await res.json();
          if (predData.predictions?.length) {
            const grouped: Record<string, unknown[]> = {};
            for (const pred of predData.predictions) {
              const key = pred.metric_type || "unknown";
              if (!grouped[key]) grouped[key] = [];
              grouped[key].push(pred);
            }

            for (const [metricType, preds] of Object.entries(grouped)) {
              const values = (preds as { value: number }[]).map((p) => p.value);
              data.predictions.push({
                serverId,
                serverName: agent.name || serverId,
                metricType,
                modelType: (preds[0] as { model_type?: string })?.model_type || "chronos",
                count: preds.length,
                avgPredicted: values.reduce((a, b) => a + b, 0) / values.length,
                minPredicted: Math.min(...values),
                maxPredicted: Math.max(...values),
              });
            }
          }
        }
      } catch {}
    }
  }

  return data;
}

export async function generateReport(
  reportType: string,
  reportConfig: ReportConfig,
  data: ReportData
): Promise<{
  success: boolean;
  content?: string;
  debugPrompt?: {
    systemPrompt: string;
    userPrompt: string;
    model: string;
    timestamp: string;
  };
  error?: string;
}> {
  const API = getApiBaseUrl();
  try {
    const res = await fetch(`${API}/api/reports/generate`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ reportType, reportConfig, data }),
    });

    if (!res.ok) {
      const err = await res.json().catch(() => ({}));
      return { success: false, error: err.error || err.details || err.message || "Error generando informe" };
    }

    const result = await res.json();
    return {
      success: true,
      content: result.content || result.report || result.message || "Informe generado",
      debugPrompt: result.debugPrompt
        ? {
            systemPrompt: result.debugPrompt.systemPrompt || result.debugPrompt.system_prompt || "",
            userPrompt: result.debugPrompt.userPrompt || result.debugPrompt.user_prompt || "",
            model: result.debugPrompt.model || "",
            timestamp: result.debugPrompt.timestamp || new Date().toISOString(),
          }
        : undefined,
    };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}
