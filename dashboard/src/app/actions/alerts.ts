"use server";

import { getApiBaseUrl } from "@/lib/api-config";
import type { DeviceThresholds } from "@/components/sections/alerts/types";

export async function getThresholds(agentId: string): Promise<DeviceThresholds | null> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/alerts/thresholds/${agentId}`, {
      cache: "no-store",
    });
    if (!res.ok) return null;
    const data = await res.json();
    if (!data.thresholds?.length) return null;

    return {
      device_id: data.device_id,
      device_name: data.device_name || agentId,
      device_type: "server",
      thresholds: data.thresholds.map(
        (t: {
          metric_name: string;
          warning_threshold: number | null;
          critical_threshold: number | null;
          comparison?: string;
        }) => ({
          metric_name: t.metric_name,
          display_name: t.metric_name
            .replace(/_/g, " ")
            .replace(/\b\w/g, (c: string) => c.toUpperCase()),
          unit: "%",
          warning: t.warning_threshold,
          critical: t.critical_threshold,
          comparison: t.comparison || "gt",
          priority: "medium",
          enabled: true,
          reasoning: "",
          ai_model: null,
        })
      ),
    };
  } catch {
    return null;
  }
}

export async function getDiskValue(agentId: string): Promise<number | null> {
  const API = getApiBaseUrl();
  const query = `from(bucket: "metrics")
    |> range(start: -5m)
    |> filter(fn: (r) => r._measurement == "metrics_v2")
    |> filter(fn: (r) => r.node_id == "${agentId}")
    |> filter(fn: (r) => r.component =~ /fs_root_usage_percent|fs_home_usage_percent|disk_percent/)
    |> last()`;

  try {
    const res = await fetch(`${API}/api/v1/metrics/query`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query }),
      cache: "no-store",
    });
    if (!res.ok) return null;
    const data = await res.json();
    const point = data.data?.find((r: { value?: number }) => r.value !== undefined);
    return point?.value ?? null;
  } catch {
    return null;
  }
}

export async function analyzeServerThresholds(
  agentId: string,
  agentName: string,
  currentMetrics?: { cpu_percent?: number | null; memory_percent?: number | null; disk_percent?: number | null }
): Promise<{ success: boolean; error?: string }> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/alerts/thresholds/analyze`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        device_id: agentId,
        device_name: agentName,
        device_type: "server",
        protocol: "agent",
        available_metrics: ["cpu_percent", "memory_percent", "disk_percent"],
        current_metrics: currentMetrics,
      }),
    });

    if (!res.ok) {
      const err: { detail?: string; message?: string } = await res.json().catch(() => ({}));
      return { success: false, error: err.detail || err.message || "Error al analizar" };
    }

    return { success: true };
  } catch {
    return { success: false, error: "Error de conexion" };
  }
}

export async function getMetricHistory(
  agentId: string,
  metric: string
): Promise<Array<{ timestamp: string; value: number }>> {
  const API = getApiBaseUrl();
  const patterns: Record<string, string> = {
    cpu_percent: "cpu",
    memory_percent: "memory_percent",
    fs_root_usage_percent: "fs_root_usage_percent|disk",
  };
  const pattern = patterns[metric] || metric;

  const query = `from(bucket: "metrics")
    |> range(start: -7d)
    |> filter(fn: (r) => r._measurement == "metrics_v2")
    |> filter(fn: (r) => r.node_id == "${agentId}")
    |> filter(fn: (r) => r.component =~ /${pattern}/)
    |> aggregateWindow(every: 5m, fn: mean, createEmpty: false)
    |> yield(name: "mean")`;

  try {
    const res = await fetch(`${API}/api/v1/metrics/query`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ query }),
      cache: "no-store",
    });
    if (!res.ok) return [];

    const data = await res.json();
    if (!data.data?.length) return [];

    const pointsByTime = new Map<string, number[]>();
    for (const p of data.data) {
      const ts = p.time || p._time;
      const val = p.value ?? p._value ?? 0;
      if (ts && !isNaN(val)) {
        if (!pointsByTime.has(ts)) pointsByTime.set(ts, []);
        pointsByTime.get(ts)!.push(val);
      }
    }

    return Array.from(pointsByTime.entries())
      .map(([ts, vals]) => ({
        timestamp: ts,
        value: vals.reduce((a, b) => a + b, 0) / vals.length,
      }))
      .sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
  } catch {
    return [];
  }
}
