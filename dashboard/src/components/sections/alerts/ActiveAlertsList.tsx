"use client";

import { AlertTriangle, XCircle } from "lucide-react";
import type { ActiveAlert } from "./types";

interface ActiveAlertsListProps {
  alerts: ActiveAlert[];
}

export function ActiveAlertsList({ alerts }: ActiveAlertsListProps) {
  if (alerts.length === 0) return null;

  return (
    <div className="space-y-2">
      <header className="flex items-center gap-2 mb-2">
        <AlertTriangle className="w-4 h-4 text-red-400" />
        <span className="text-[13px] font-medium text-white">
          Alertas Activas ({alerts.length})
        </span>
      </header>
      
      <ul className="space-y-2">
        {alerts.map(alert => (
          <AlertItem key={alert.id} alert={alert} />
        ))}
      </ul>
    </div>
  );
}

function AlertItem({ alert }: { alert: ActiveAlert }) {
  const isCritical = alert.severity === "critical";
  
  return (
    <li
      className={`p-3 rounded-lg border ${
        isCritical
          ? "bg-red-500/10 border-red-500/30"
          : "bg-yellow-500/10 border-yellow-500/30"
      }`}
    >
      <div className="flex items-center justify-between">
        <div className="flex items-center gap-2">
          {isCritical ? (
            <XCircle className="w-4 h-4 text-red-400" />
          ) : (
            <AlertTriangle className="w-4 h-4 text-yellow-400" />
          )}
          <span className="text-[13px] font-medium text-white">
            {alert.server_name}
          </span>
          <span className="text-[11px] text-[var(--color-text-secondary)]">|</span>
          <span className="text-[12px] text-[var(--color-text-secondary)]">
            {alert.display_name}
          </span>
        </div>
        <span
          className={`text-[12px] font-medium ${
            isCritical ? "text-red-400" : "text-yellow-400"
          }`}
        >
          {alert.current_value.toFixed(1)}{alert.unit}
          <span className="text-[var(--color-text-secondary)] font-normal">
            {" "}(limite: {alert.threshold_value}{alert.unit})
          </span>
        </span>
      </div>
    </li>
  );
}




