"use client";

import { useEffect, useRef } from "react";
import { Shield, Loader2, CheckCircle, XCircle, TrendingUp } from "lucide-react";
import { useAlertsThresholds } from "@/hooks/useAlertsThresholds";
import {
  ActiveAlertsList,
  NetworkStats,
  ThresholdsList,
  UnconfiguredList,
  PredictionsPanel,
  type Agent,
  type PredictedAnomaly,
} from "./alerts";

interface AlertsThresholdsSectionProps {
  agents: Agent[];
}

export function AlertsThresholdsSection({ agents }: AlertsThresholdsSectionProps) {
  const state = useAlertsThresholds(agents);
  const {
    thresholds,
    activeAlerts,
    predictions,
    cachedPredictions,
    loading,
    analyzing,
    predicting,
    error,
    predictionError,
    chronosOnline,
    expanded,
    showPredictions,
    metrics,
    unconfigured,
    agentIds,
    loadInitialData,
    analyzeServer,
    runPrediction,
    toggleExpanded,
    closePredictions,
  } = state;

  const initializedAgentsRef = useRef("");

  // Load data when agents change (mirrors original behavior)
  useEffect(() => {
    if (agents.length === 0) return;
    if (initializedAgentsRef.current === agentIds) return;
    
    initializedAgentsRef.current = agentIds;
    loadInitialData();
  }, [agents.length, agentIds, loadInitialData]);

  if (loading) {
    return (
      <div className="flex items-center justify-center py-8">
        <Loader2 className="w-5 h-5 animate-spin text-emerald-400 mr-2" />
        <span className="text-[13px] text-[var(--color-text-secondary)]">
          Cargando configuracion de alertas...
        </span>
      </div>
    );
  }

  const hasThresholds = thresholds.length > 0;
  const noAlerts = activeAlerts.length === 0 && hasThresholds;

  return (
    <div className="space-y-4">
      {error && <ErrorBanner message={error} />}

      <ActiveAlertsList alerts={activeAlerts} />

      {cachedPredictions.length > 0 && (
        <UpcomingBreachPanel predictions={cachedPredictions} />
      )}

      {noAlerts && (
        <div className="flex items-center justify-center gap-2 py-4 text-[var(--color-text-secondary)]">
          <CheckCircle className="w-4 h-4 text-emerald-400" />
          <span className="text-[13px]">
            Todos los dispositivos están dentro de los limites configurados
          </span>
        </div>
      )}

      <NetworkStats agents={agents} />

      {hasThresholds && (
        <ThresholdsSection
          thresholds={thresholds}
          agents={agents}
          metrics={metrics}
          expanded={expanded}
          analyzing={analyzing}
          predicting={predicting}
          chronosOnline={chronosOnline}
          showPredictions={showPredictions}
          predictions={predictions}
          predictionError={predictionError}
          onToggle={toggleExpanded}
          onAnalyze={analyzeServer}
          onPredict={runPrediction}
          onClosePredictions={closePredictions}
        />
      )}

      <UnconfiguredList agents={unconfigured} analyzing={analyzing} onAnalyze={analyzeServer} />

      {!hasThresholds && unconfigured.length === 0 && <EmptyState />}
    </div>
  );
}


function ErrorBanner({ message }: { message: string }) {
  return (
    <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 flex items-center gap-2">
      <XCircle className="w-4 h-4 text-red-400" />
      <span className="text-[13px] text-red-400">{message}</span>
    </div>
  );
}

function EmptyState() {
  return (
    <div className="text-center py-8 text-[var(--color-text-secondary)]">
      <Shield className="w-8 h-8 mx-auto mb-2 opacity-50" />
      <p className="text-[13px]">No hay servidores para configurar alertas</p>
    </div>
  );
}

interface ThresholdsSectionProps {
  thresholds: ReturnType<typeof useAlertsThresholds>["thresholds"];
  agents: Agent[];
  metrics: ReturnType<typeof useAlertsThresholds>["metrics"];
  expanded: Set<string>;
  analyzing: string | null;
  predicting: boolean;
  chronosOnline: boolean | null;
  showPredictions: boolean;
  predictions: ReturnType<typeof useAlertsThresholds>["predictions"];
  predictionError: string | null;
  onToggle: (id: string) => void;
  onAnalyze: (agent: Agent) => void;
  onPredict: () => void;
  onClosePredictions: () => void;
}

function ThresholdsSection({
  thresholds,
  agents,
  metrics,
  expanded,
  analyzing,
  predicting,
  chronosOnline,
  showPredictions,
  predictions,
  predictionError,
  onToggle,
  onAnalyze,
  onPredict,
  onClosePredictions,
}: ThresholdsSectionProps) {
  const canPredict = !predicting && chronosOnline && thresholds.length > 0;

  return (
    <div className="space-y-3">
      <header className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          <Shield className="w-4 h-4 text-emerald-400" />
          <span className="text-[13px] font-medium text-white">Umbrales</span>
        </div>
        <button
          onClick={onPredict}
          disabled={!canPredict}
          className="flex items-center gap-1.5 px-3 py-1.5 bg-cyan-500/20 hover:bg-cyan-500/30 text-cyan-400 rounded-lg text-[12px] font-medium disabled:opacity-50"
        >
          {predicting ? (
            <Loader2 className="w-3.5 h-3.5 animate-spin" />
          ) : (
            <TrendingUp className="w-3.5 h-3.5" />
          )}
          {predicting ? "Analizando..." : "Predecir"}
        </button>
      </header>

      {showPredictions && (
        <PredictionsPanel predictions={predictions} error={predictionError} onClose={onClosePredictions} />
      )}

      <ThresholdsList
        thresholds={thresholds}
        agents={agents}
        metrics={metrics}
        expanded={expanded}
        analyzing={analyzing}
        onToggle={onToggle}
        onAnalyze={onAnalyze}
      />
    </div>
  );
}

function UpcomingBreachPanel({ predictions }: { predictions: PredictedAnomaly[] }) {
  const formatDate = (iso: string) => {
    try {
      return new Date(iso).toLocaleString("es-ES", {
        day: "2-digit",
        month: "short",
        hour: "2-digit",
        minute: "2-digit",
      });
    } catch {
      return iso;
    }
  };

  return (
    <div className="rounded-lg border border-amber-500/30 bg-amber-500/5 overflow-hidden">
      <table className="w-full text-[12px]">
        <thead>
          <tr className="bg-amber-500/10 text-[var(--color-text-secondary)]">
            <th className="text-left p-3 font-medium">Servidor</th>
            <th className="text-left p-3 font-medium">Métrica</th>
            <th className="text-center p-3 font-medium">Valor Predicho</th>
            <th className="text-center p-3 font-medium">Fecha Estimada</th>
          </tr>
        </thead>
        <tbody>
          {predictions.map((p) => (
            <tr key={p.id} className="border-t border-amber-500/10">
              <td className="p-3 text-white">{p.server_name}</td>
              <td className="p-3 text-[var(--color-text-secondary)]">{p.display_name}</td>
              <td className="p-3 text-center text-amber-400 font-medium">{p.predicted_value}%</td>
              <td className="p-3 text-center text-amber-400">{formatDate(p.predicted_at)}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}
