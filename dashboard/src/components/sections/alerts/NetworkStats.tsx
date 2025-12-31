"use client";

import { Activity, ArrowUpFromLine, ArrowDownToLine } from "lucide-react";
import { formatBytes, formatSpeed } from "./utils";
import type { Agent } from "./types";

interface NetworkStatsProps {
  agents: Agent[];
}

export function NetworkStats({ agents }: NetworkStatsProps) {
  const networkedAgents = agents.filter(a => a.network_tx_rate || a.network_rx_rate);
  
  if (networkedAgents.length === 0) return null;

  return (
    <div className="space-y-3">
      <header className="flex items-center gap-2">
        <Activity className="w-4 h-4 text-cyan-400" />
        <span className="text-[13px] font-medium text-white">Red</span>
      </header>

      <div className="grid grid-cols-1 lg:grid-cols-2 gap-3">
        {networkedAgents.map(agent => (
          <NetworkCard key={`net-${agent.id}`} agent={agent} />
        ))}
      </div>
    </div>
  );
}

function NetworkCard({ agent }: { agent: Agent }) {
  return (
    <article className="p-4 rounded-lg border border-[var(--color-border)] bg-[var(--color-bg-primary)]">
      <header className="flex items-center gap-2 mb-4">
        <div className="w-2 h-2 rounded-full bg-cyan-400" />
        <span className="text-[13px] font-medium text-white truncate">
          {agent.name || agent.id}
        </span>
      </header>

      <div className="grid grid-cols-2 gap-4 mb-4">
        <div className="p-3 rounded-lg bg-emerald-500/5 border border-emerald-500/20">
          <div className="flex items-center gap-2 mb-2">
            <ArrowUpFromLine className="w-4 h-4 text-emerald-400" />
            <span className="text-[11px] text-emerald-400/70 uppercase">Subida</span>
          </div>
          <div className="text-lg font-bold text-emerald-400">
            {formatSpeed(agent.network_tx_rate || 0)}
          </div>
        </div>

        <div className="p-3 rounded-lg bg-blue-500/5 border border-blue-500/20">
          <div className="flex items-center gap-2 mb-2">
            <ArrowDownToLine className="w-4 h-4 text-blue-400" />
            <span className="text-[11px] text-blue-400/70 uppercase">Bajada</span>
          </div>
          <div className="text-lg font-bold text-blue-400">
            {formatSpeed(agent.network_rx_rate || 0)}
          </div>
        </div>
      </div>

      <footer className="flex items-center justify-between pt-3 border-t border-[var(--color-border)] text-[11px] text-[var(--color-text-secondary)]">
        <span>
          TX: <span className="text-emerald-400/80">{formatBytes(agent.network_tx_bytes || 0)}</span>
        </span>
        <span>
          RX: <span className="text-blue-400/80">{formatBytes(agent.network_rx_bytes || 0)}</span>
        </span>
      </footer>
    </article>
  );
}




