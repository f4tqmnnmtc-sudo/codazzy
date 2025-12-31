"use client";

import { Shield, Brain, Loader2, ChevronDown, ChevronUp } from "lucide-react";
import { getMetricIcon, getMetricStatus } from "./utils";
import type { DeviceThresholds, ThresholdConfig, MetricValues, Agent } from "./types";

interface ThresholdsListProps {
  thresholds: DeviceThresholds[];
  agents: Agent[];
  metrics: Record<string, MetricValues>;
  expanded: Set<string>;
  analyzing: string | null;
  onToggle: (id: string) => void;
  onAnalyze: (agent: Agent) => void;
}

export function ThresholdsList({
  thresholds,
  agents,
  metrics,
  expanded,
  analyzing,
  onToggle,
  onAnalyze,
}: ThresholdsListProps) {
  if (thresholds.length === 0) return null;

  return (
    <div className="space-y-3">
      {thresholds.map(device => {
        const agent = agents.find(a => a.id === device.device_id);
        const isExpanded = expanded.has(device.device_id);

        return (
          <ThresholdCard
            key={device.device_id}
            device={device}
            metrics={metrics[device.device_id]}
            isExpanded={isExpanded}
            isAnalyzing={analyzing === device.device_id}
            onToggle={() => onToggle(device.device_id)}
            onAnalyze={() => agent && onAnalyze(agent)}
          />
        );
      })}
    </div>
  );
}

interface ThresholdCardProps {
  device: DeviceThresholds;
  metrics?: MetricValues;
  isExpanded: boolean;
  isAnalyzing: boolean;
  onToggle: () => void;
  onAnalyze: () => void;
}

function ThresholdCard({
  device,
  metrics,
  isExpanded,
  isAnalyzing,
  onToggle,
  onAnalyze,
}: ThresholdCardProps) {
  return (
    <article className="rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)] overflow-hidden">
      <div className="flex items-center justify-between p-3 hover:bg-[var(--color-bg-secondary)] transition-colors">
        <button
          className="flex items-center gap-2 flex-1"
          onClick={onToggle}
        >
          <div className="w-2 h-2 rounded-full bg-emerald-400" />
          <span className="text-[13px] font-medium text-white">{device.device_name}</span>
          <span className="text-[11px] text-[var(--color-text-secondary)] bg-[var(--color-bg-tertiary)] px-2 py-0.5 rounded">
            {device.thresholds.length} metricas
          </span>
        </button>

        <div className="flex items-center gap-2">
          <button
            onClick={e => { e.stopPropagation(); onAnalyze(); }}
            disabled={isAnalyzing}
            className="flex items-center gap-1 px-2 py-1 text-[11px] text-purple-400 hover:bg-purple-500/10 rounded transition-colors disabled:opacity-50"
          >
            {isAnalyzing ? (
              <Loader2 className="w-3 h-3 animate-spin" />
            ) : (
              <Brain className="w-3 h-3" />
            )}
            Generar Umbrales
          </button>
          
          <button onClick={onToggle} className="p-1">
            {isExpanded ? (
              <ChevronUp className="w-4 h-4 text-[var(--color-text-secondary)]" />
            ) : (
              <ChevronDown className="w-4 h-4 text-[var(--color-text-secondary)]" />
            )}
          </button>
        </div>
      </div>

      {isExpanded && metrics && (
        <ThresholdTable thresholds={device.thresholds} metrics={metrics} />
      )}
    </article>
  );
}

function ThresholdTable({ thresholds, metrics }: { thresholds: ThresholdConfig[]; metrics: MetricValues }) {
  return (
    <div className="border-t border-[var(--color-border)]">
      <table className="w-full text-[12px]">
        <thead>
          <tr className="bg-[var(--color-bg-secondary)]">
            <th className="text-left p-2 text-[var(--color-text-secondary)] font-medium">Metrica</th>
            <th className="text-center p-2 text-yellow-400 font-medium" />
            <th className="text-center p-2 text-red-400 font-medium" />
            <th className="text-center p-2 text-[var(--color-text-secondary)] font-medium">Estado Actual</th>
          </tr>
        </thead>
        <tbody>
          {thresholds.map((threshold, idx) => {
            const status = getMetricStatus(metrics, threshold);
            const Icon = getMetricIcon(threshold.metric_name);

            return (
              <tr key={idx} className="border-t border-[var(--color-border)] hover:bg-[var(--color-bg-secondary)]/50">
                <td className="p-2">
                  <div className="flex items-center gap-2">
                    <Icon className="w-4 h-4" />
                    <span className="text-white">{threshold.display_name}</span>
                  </div>
                </td>
                <td className="text-center p-2 text-yellow-400">
                  {threshold.warning !== null ? `${threshold.warning}${threshold.unit}` : "-"}
                </td>
                <td className="text-center p-2 text-red-400">
                  {threshold.critical !== null ? `${threshold.critical}${threshold.unit}` : "-"}
                </td>
                <td className="text-center p-2">
                  <StatusValue status={status} unit={threshold.unit} />
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}

function StatusValue({ status, unit }: { status: { status: string; value: number | null }; unit: string }) {
  const colors: Record<string, string> = {
    ok: "text-emerald-400",
    warning: "text-yellow-400",
    critical: "text-red-400",
    unknown: "text-[var(--color-text-secondary)]",
  };

  if (status.value === null) {
    return <span className={colors.unknown}>-</span>;
  }

  return (
    <span className={colors[status.status]}>
      {status.value.toFixed(1)}{unit}
    </span>
  );
}

interface UnconfiguredListProps {
  agents: Agent[];
  analyzing: string | null;
  onAnalyze: (agent: Agent) => void;
}

export function UnconfiguredList({ agents, analyzing, onAnalyze }: UnconfiguredListProps) {
  if (agents.length === 0) return null;

  return (
    <div className="space-y-3">
      <header className="flex items-center gap-2">
        <Shield className="w-4 h-4 text-[var(--color-text-secondary)]" />
        <span className="text-[13px] font-medium text-white">Servidores sin configurar</span>
      </header>

      {agents.map(agent => (
        <div
          key={agent.id}
          className="p-3 rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)] flex items-center justify-between"
        >
          <div className="flex items-center gap-2">
            <div className="w-2 h-2 rounded-full bg-[var(--color-text-secondary)]" />
            <span className="text-[13px] text-white">{agent.name || agent.id}</span>
            <span className="text-[11px] text-[var(--color-text-secondary)]">Sin umbrales</span>
          </div>
          
          <button
            onClick={() => onAnalyze(agent)}
            disabled={analyzing === agent.id}
            className="flex items-center gap-2 px-3 py-1.5 bg-purple-500/20 hover:bg-purple-500/30 text-purple-400 rounded-lg text-[12px] font-medium disabled:opacity-50"
          >
            {analyzing === agent.id ? (
              <>
                <Loader2 className="w-4 h-4 animate-spin" />
                Analizando...
              </>
            ) : (
              <>
                <Brain className="w-4 h-4" />
                Configurar
              </>
            )}
          </button>
        </div>
      ))}
    </div>
  );
}




