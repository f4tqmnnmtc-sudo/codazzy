import { Cpu, Activity, HardDrive, Thermometer, type LucideIcon } from "lucide-react";
import type { ThresholdConfig, MetricValues, MetricStatus } from "./types";

export function getMetricIcon(name: string): LucideIcon {
  const n = name.toLowerCase();
  if (n.includes("cpu")) return Cpu;
  if (n.includes("mem")) return Activity;
  if (n.includes("disk")) return HardDrive;
  if (n.includes("temp")) return Thermometer;
  return Activity;
}

export function formatBytes(bytes: number): string {
  if (bytes <= 0) return "0 B";
  const units = ["B", "KB", "MB", "GB", "TB"];
  const i = Math.floor(Math.log(bytes) / Math.log(1024));
  return `${(bytes / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function formatSpeed(bps: number): string {
  if (bps <= 0) return "0 B/s";
  const units = ["B/s", "KB/s", "MB/s", "GB/s"];
  const i = Math.min(Math.floor(Math.log(bps) / Math.log(1024)), units.length - 1);
  return `${(bps / Math.pow(1024, i)).toFixed(1)} ${units[i]}`;
}

export function getMetricStatus(
  metrics: MetricValues,
  threshold: ThresholdConfig
): MetricStatus {
  const name = threshold.metric_name.toLowerCase();
  let val: number | null = null;
  
  if (name.includes("cpu")) val = metrics.cpu;
  else if (name.includes("mem")) val = metrics.mem;
  else if (name.includes("disk")) val = metrics.disk;
  
  if (val === null) return { status: "unknown", value: null };

  const above = threshold.comparison === "gt" || threshold.comparison === "gte";
  
  if (threshold.critical !== null && (above ? val >= threshold.critical : val <= threshold.critical)) {
    return { status: "critical", value: val };
  }
  if (threshold.warning !== null && (above ? val >= threshold.warning : val <= threshold.warning)) {
    return { status: "warning", value: val };
  }
  
  return { status: "ok", value: val };
}

export function getMetricValue(metrics: MetricValues, metricName: string): number | null {
  const name = metricName.toLowerCase();
  if (name.includes("cpu")) return metrics.cpu;
  if (name.includes("mem")) return metrics.mem;
  if (name.includes("disk")) return metrics.disk;
  return null;
}

