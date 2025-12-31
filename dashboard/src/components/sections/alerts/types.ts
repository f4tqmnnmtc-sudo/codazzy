import type { Agent } from "@/types/gateway";

export type { Agent };

export interface ThresholdConfig {
  metric_name: string;
  display_name: string;
  unit: string;
  warning: number | null;
  critical: number | null;
  comparison: string;
  priority: string;
  enabled: boolean;
  reasoning: string;
  ai_model: string | null;
}

export interface DeviceThresholds {
  device_id: string;
  device_name: string;
  device_type: string;
  thresholds: ThresholdConfig[];
}

export interface ActiveAlert {
  id: string;
  server_id: string;
  server_name: string;
  metric_name: string;
  display_name: string;
  current_value: number;
  threshold_value: number;
  severity: "warning" | "critical";
  comparison: string;
  unit: string;
  detected_at: string;
}

export interface PredictedAnomaly {
  id: string;
  server_id: string;
  server_name: string;
  metric_name: string;
  display_name: string;
  predicted_value: number;
  threshold_value: number;
  threshold_type: "warning" | "critical";
  predicted_at: string;
  confidence: number;
  current_value: number;
  trend: "increasing" | "decreasing" | "stable";
  hours_until: number;
}

export interface MetricValues {
  cpu: number | null;
  mem: number | null;
  disk: number | null;
}

export type MetricStatus = {
  status: 'ok' | 'warning' | 'critical' | 'unknown';
  value: number | null;
};




