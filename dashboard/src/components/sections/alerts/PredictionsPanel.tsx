"use client";

import { TrendingUp, XCircle } from "lucide-react";
import type { PredictedAnomaly } from "./types";

interface PredictionsPanelProps {
  predictions: PredictedAnomaly[];
  error: string | null;
  onClose: () => void;
}

export function PredictionsPanel({ predictions, error, onClose }: PredictionsPanelProps) {
  return (
    <div className="rounded-lg border border-cyan-500/30 bg-cyan-500/5 overflow-hidden">
      <header
        className="flex items-center justify-between p-3 cursor-pointer"
        onClick={onClose}
      >
        <div className="flex items-center gap-2">
          <TrendingUp className="w-4 h-4 text-cyan-400" />
          <span className="text-[13px] font-medium text-white">Predicciones</span>
        </div>
        <button
          onClick={e => { e.stopPropagation(); onClose(); }}
          className="text-[var(--color-text-secondary)] hover:text-white"
        >
          <XCircle className="w-4 h-4" />
        </button>
      </header>

      {error && (
        <div className="px-3 pb-3">
          <div className="p-2 rounded bg-red-500/10 text-red-400 text-[12px]">{error}</div>
        </div>
      )}

      {predictions.length > 0 ? (
        <PredictionsTable predictions={predictions} />
      ) : !error && (
        <div className="border-t border-cyan-500/20 p-4 text-center text-[12px] text-[var(--color-text-secondary)]">
          No se predicen anomalías en las próximas horas
        </div>
      )}
    </div>
  );
}

function PredictionsTable({ predictions }: { predictions: PredictedAnomaly[] }) {
  return (
    <div className="border-t border-cyan-500/20">
      <table className="w-full text-[12px]">
        <thead>
          <tr className="bg-[var(--color-bg-secondary)] text-[var(--color-text-secondary)]">
            <th className="text-left p-2 font-medium">Servidor</th>
            <th className="text-left p-2 font-medium">Métrica</th>
            <th className="text-center p-2 font-medium">Actual</th>
            <th className="text-center p-2 font-medium">Predicción</th>
            <th className="text-center p-2 font-medium">Límite</th>
            <th className="text-center p-2 font-medium">Predicción</th>
            <th className="text-center p-2 font-medium">Prob.</th>
          </tr>
        </thead>
        <tbody>
          {predictions.map(p => (
            <PredictionRow key={p.id} prediction={p} />
          ))}
        </tbody>
      </table>
    </div>
  );
}

function PredictionRow({ prediction: p }: { prediction: PredictedAnomaly }) {
  const active = p.hours_until === 0;
  const noBreach = p.hours_until < 0;

  const predictionColor = active
    ? "text-red-400"
    : noBreach
      ? "text-emerald-400"
      : "text-amber-400";

  const timeColor = active
    ? "text-red-400"
    : noBreach
      ? "text-emerald-400"
      : "text-cyan-400";

  const formatTime = () => {
    if (active) return "ahora";
    if (noBreach) return "OK";
    if (p.hours_until < 1) return `${Math.round(p.hours_until * 60)} min`;
    return `${p.hours_until.toFixed(1)}h`;
  };

  return (
    <tr className={`border-t border-[var(--color-border)] ${active ? "bg-red-500/5" : ""}`}>
      <td className="p-2 text-white">{p.server_name}</td>
      <td className="p-2 text-[var(--color-text-secondary)]">{p.display_name}</td>
      <td className="p-2 text-center text-white">{p.current_value}%</td>
      <td className={`p-2 text-center ${predictionColor}`}>{p.predicted_value}%</td>
      <td className="p-2 text-center text-[var(--color-text-secondary)]">{p.threshold_value}%</td>
      <td className={`p-2 text-center ${timeColor}`}>{formatTime()}</td>
      <td className="p-2 text-center text-[var(--color-text-secondary)]">{p.confidence}%</td>
    </tr>
  );
}




