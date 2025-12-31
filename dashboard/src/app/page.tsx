"use client";

import { useReducer, useCallback, useRef, useEffect } from "react";
import { ServerCard, StatCard, GaugeCard } from "@/components/cards";
import { Accordion, SidePanel, ServerDetailPanel, TopHeader, MiniSidebar } from "@/components/layout";
import { 
  AgentInstallSection, 
  ConfigEditorSection, 
  PredictiveModelsSection, 
  AIReportSection, 
  NetworkDevicesSection, 
  NetworkDiscoverySection, 
  AlertsThresholdsSection 
} from "@/components/sections";
import { getApiBaseUrl } from "@/lib/api-config";
import type { Agent, ServerConnection } from "@/types/gateway";

const api = getApiBaseUrl();

type DashboardState = {
  agents: Agent[];
  connections: Map<string, ServerConnection>;
  loading: boolean;
  lastUpdate: Date | null;
  selectedServer: Agent | null;
  panelTab: string;
  activeSection: string;
  panels: {
    install: boolean;
    config: boolean;
    prediction: boolean;
    report: boolean;
  };
  configServerId: string | null;
};

type DashboardAction =
  | { type: 'SET_AGENTS'; payload: Agent[] }
  | { type: 'SET_CONNECTIONS'; payload: Map<string, ServerConnection> }
  | { type: 'SET_LOADING'; payload: boolean }
  | { type: 'UPDATE_TIMESTAMP' }
  | { type: 'SELECT_SERVER'; payload: Agent | null }
  | { type: 'SET_PANEL_TAB'; payload: string }
  | { type: 'SET_ACTIVE_SECTION'; payload: string }
  | { type: 'TOGGLE_PANEL'; panel: keyof DashboardState['panels']; value: boolean }
  | { type: 'SET_CONFIG_SERVER'; payload: string | null }
  | { type: 'UPDATE_SELECTED_SERVER'; payload: Agent };

const initialState: DashboardState = {
  agents: [],
  connections: new Map(),
  loading: true,
  lastUpdate: null,
  selectedServer: null,
  panelTab: "Resumen",
  activeSection: "top",
  panels: { install: false, config: false, prediction: false, report: false },
  configServerId: null,
};

function dashboardReducer(state: DashboardState, action: DashboardAction): DashboardState {
  switch (action.type) {
    case 'SET_AGENTS':
      return { ...state, agents: action.payload };
    case 'SET_CONNECTIONS':
      return { ...state, connections: action.payload };
    case 'SET_LOADING':
      return { ...state, loading: action.payload };
    case 'UPDATE_TIMESTAMP':
      return { ...state, lastUpdate: new Date() };
    case 'SELECT_SERVER':
      return { ...state, selectedServer: action.payload, panelTab: "Resumen" };
    case 'SET_PANEL_TAB':
      return { ...state, panelTab: action.payload };
    case 'SET_ACTIVE_SECTION':
      return { ...state, activeSection: action.payload };
    case 'TOGGLE_PANEL':
      return { ...state, panels: { ...state.panels, [action.panel]: action.value } };
    case 'SET_CONFIG_SERVER':
      return { ...state, configServerId: action.payload };
    case 'UPDATE_SELECTED_SERVER':
      return state.selectedServer?.id === action.payload.id 
        ? { ...state, selectedServer: action.payload }
        : state;
    default:
      return state;
  }
}

async function fetchDashboardData(skipCache: boolean): Promise<{
  agents: Agent[];
  connections: Map<string, ServerConnection>;
}> {
  if (skipCache) {
    await fetch(`${api}/api/v1/metrics/cache/clear`, { method: 'POST' }).catch(() => {});
  }
  
  const [agentsRes, serversRes] = await Promise.all([
    fetch(`${api}/api/v1/metrics/agents`).catch(() => null),
    fetch(`${api}/api/v1/agents/installed-servers`).catch(() => null),
  ]);
  
  let agents: Agent[] = [];
  if (agentsRes?.ok) {
    const data = await agentsRes.json();
    agents = data.agents || [];
  }
  
  const connections = new Map<string, ServerConnection>();
  if (serversRes?.ok) {
    const data = await serversRes.json();
    if (Array.isArray(data)) {
      data.forEach((s: ServerConnection) => connections.set(s.node_id, s));
    }
  }
  
  return { agents, connections };
}

export default function DashboardPage() {
  const [state, dispatch] = useReducer(dashboardReducer, initialState);
  const selectedServerRef = useRef<Agent | null>(null);
  const intervalRef = useRef<ReturnType<typeof setInterval>>();

  const fetchData = useCallback(async (skipCache = false) => {
    try {
      const { agents, connections } = await fetchDashboardData(skipCache);
      
      dispatch({ type: 'SET_AGENTS', payload: agents });
      dispatch({ type: 'SET_CONNECTIONS', payload: connections });
      dispatch({ type: 'UPDATE_TIMESTAMP' });
      
      const current = selectedServerRef.current;
      if (current) {
        const updated = agents.find(a => a.id === current.id);
        if (updated) {
          dispatch({ type: 'UPDATE_SELECTED_SERVER', payload: updated });
          selectedServerRef.current = updated;
        }
      }
    } catch (e) {
      console.error("fetch err:", e);
    } finally {
      dispatch({ type: 'SET_LOADING', payload: false });
    }
  }, []);

  useEffect(() => {
    fetchData();
    intervalRef.current = setInterval(() => fetchData(false), 5000);
    return () => {
      if (intervalRef.current) clearInterval(intervalRef.current);
    };
  }, [fetchData]);

  const handleSelectServer = useCallback((agent: Agent) => {
    selectedServerRef.current = agent;
    dispatch({ type: 'SELECT_SERVER', payload: agent });
  }, []);

  const handleCloseServer = useCallback(() => {
    selectedServerRef.current = null;
    dispatch({ type: 'SELECT_SERVER', payload: null });
  }, []);

  const { agents, connections, loading, lastUpdate, selectedServer, panelTab, activeSection, panels, configServerId } = state;
  
  const online = agents.filter(a => a.status === "online");
  const offline = agents.filter(a => a.status !== "online");
  const avgCpu = agents.length ? Math.round(agents.reduce((s, a) => s + (a.cpu_usage || 0), 0) / agents.length) : 0;
  const avgMem = agents.length ? Math.round(agents.reduce((s, a) => s + (a.memory_usage || 0), 0) / agents.length) : 0;
  const healthPct = agents.length ? Math.round((online.length / agents.length) * 100) : 0;

  return (
    <div className="min-h-screen bg-[var(--color-bg-primary)] text-white">
      <MiniSidebar activeSection={activeSection} onSectionClick={s => dispatch({ type: 'SET_ACTIVE_SECTION', payload: s })} />
      
      <div className="ml-14">
        <TopHeader onRefresh={() => fetchData(true)} lastUpdate={lastUpdate} />
        
        <main className="p-6 space-y-4">
          <StatsGrid 
            total={agents.length}
            online={online.length}
            offline={offline.length}
            healthPct={healthPct}
            avgCpu={avgCpu}
            avgMem={avgMem}
          />

          <section id="alerts" className="scroll-mt-20">
            <Accordion 
              title="Problemas y Alertas" 
              icon={<AlertIcon />} 
              badge={0}
              badgeVariant="default"
              defaultOpen
            >
              <AlertsThresholdsSection agents={agents} />
            </Accordion>
          </section>

          <section id="infrastructure" className="scroll-mt-20">
            <Accordion 
              title="Infraestructura" 
              icon={<InfraIcon />} 
              badge={agents.length} 
              defaultOpen
              actions={
                <div className="flex gap-2">
                  <button 
                    onClick={e => { e.stopPropagation(); dispatch({ type: 'TOGGLE_PANEL', panel: 'install', value: true }); }}
                    className="px-3 py-1.5 bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-[var(--color-bg-primary)] text-[12px] font-medium rounded-lg transition-colors"
                  >
                    Instalar Agente
                  </button>
                  <button 
                    onClick={e => { e.stopPropagation(); dispatch({ type: 'TOGGLE_PANEL', panel: 'config', value: true }); }}
                    className="px-3 py-1.5 bg-[var(--color-bg-tertiary)] hover:bg-[var(--color-border)] text-white text-[12px] font-medium rounded-lg border border-[var(--color-border)] transition-colors"
                  >
                    Editar Configuracion
                  </button>
                </div>
              }
            >
              <InfrastructureContent 
                loading={loading}
                agents={agents}
                onSelectServer={handleSelectServer}
                onInstall={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'install', value: true })}
              />
            </Accordion>
          </section>

          <section id="network" className="scroll-mt-20">
            <Accordion title="Conexiones remotas" icon={<NetworkIcon />} defaultOpen={false}>
              <div className="grid grid-cols-1 lg:grid-cols-2 gap-4">
                <div className="bg-[var(--color-bg-primary)] rounded-lg p-4 border border-[var(--color-border)]">
                  <h4 className="text-[14px] font-medium text-white mb-4">Dispositivos de Red</h4>
                  <NetworkDevicesSection />
                </div>
                <div className="bg-[var(--color-bg-primary)] rounded-lg p-4 border border-[var(--color-border)]">
                  <h4 className="text-[14px] font-medium text-white mb-4">Descubrimiento de Red</h4>
                  <NetworkDiscoverySection />
                </div>
              </div>
            </Accordion>
          </section>

          <section id="ai" className="scroll-mt-20">
            <Accordion title="Analisis" icon={<AnalysisIcon />} defaultOpen>
              <AnalysisCards 
                onPrediction={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'prediction', value: true })}
                onReport={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'report', value: true })}
              />
            </Accordion>
          </section>
        </main>
      </div>

      {selectedServer && (
        <ServerDetailPanel
          agent={selectedServer}
          connection={connections.get(selectedServer.id)}
          activeTab={panelTab}
          onTabChange={t => dispatch({ type: 'SET_PANEL_TAB', payload: t })}
          onClose={handleCloseServer}
          onGenerateReport={() => {
            handleCloseServer();
            dispatch({ type: 'TOGGLE_PANEL', panel: 'report', value: true });
          }}
          onOpenConfig={() => {
            dispatch({ type: 'SET_CONFIG_SERVER', payload: selectedServer.id });
            dispatch({ type: 'TOGGLE_PANEL', panel: 'config', value: true });
          }}
        />
      )}

      <SidePanel 
        isOpen={panels.install} 
        onClose={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'install', value: false })} 
        title="Instalar Agente"
      >
        <AgentInstallSection onInstallComplete={() => fetchData(true)} />
      </SidePanel>

      <SidePanel 
        isOpen={panels.config} 
        onClose={() => { 
          dispatch({ type: 'TOGGLE_PANEL', panel: 'config', value: false }); 
          dispatch({ type: 'SET_CONFIG_SERVER', payload: null }); 
        }} 
        title="Editor de Configuracion"
      >
        <ConfigEditorSection serverId={configServerId || undefined} />
      </SidePanel>

      <SidePanel 
        isOpen={panels.prediction} 
        onClose={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'prediction', value: false })} 
        title="Chronos"
      >
        <PredictiveModelsSection />
      </SidePanel>

      <SidePanel 
        isOpen={panels.report} 
        onClose={() => dispatch({ type: 'TOGGLE_PANEL', panel: 'report', value: false })} 
        title="Generar Informe"
      >
        <AIReportSection agents={agents} />
      </SidePanel>
    </div>
  );
}

function StatsGrid({ total, online, offline, healthPct, avgCpu, avgMem }: {
  total: number;
  online: number;
  offline: number;
  healthPct: number;
  avgCpu: number;
  avgMem: number;
}) {
  return (
    <div id="top" className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-4 gap-4 scroll-mt-20">
      <StatCard label="Total Dispositivos" value={total}>
        <div className="h-8 mt-2">
          <svg viewBox="0 0 100 32" preserveAspectRatio="none" className="w-full h-full">
            <defs>
              <linearGradient id="sparkGrad" x1="0%" y1="0%" x2="0%" y2="100%">
                <stop offset="0%" stopColor="var(--color-accent)" stopOpacity="0.3"/>
                <stop offset="100%" stopColor="var(--color-accent)" stopOpacity="0"/>
              </linearGradient>
            </defs>
            <path fill="url(#sparkGrad)" d="M0,28 L10,24 L20,26 L30,20 L40,22 L50,16 L60,18 L70,12 L80,14 L90,8 L100,10 L100,32 L0,32 Z"/>
            <path fill="none" stroke="var(--color-accent)" strokeWidth="2" d="M0,28 L10,24 L20,26 L30,20 L40,22 L50,16 L60,18 L70,12 L80,14 L90,8 L100,10"/>
          </svg>
        </div>
      </StatCard>
      
      <StatCard label="Estado" value="">
        <div className="flex items-center gap-4 -mt-2">
          <div className="flex items-center gap-1.5">
            <div className="w-2.5 h-2.5 rounded-full bg-emerald-400"/>
            <span className="text-[28px] font-semibold text-emerald-400">{online}</span>
            <span className="text-[11px] text-[var(--color-text-secondary)] ml-0.5">online</span>
          </div>
          <div className="flex items-center gap-1.5">
            <div className="w-2.5 h-2.5 rounded-full bg-red-400"/>
            <span className="text-[28px] font-semibold text-red-400">{offline}</span>
            <span className="text-[11px] text-[var(--color-text-secondary)] ml-0.5">offline</span>
          </div>
        </div>
        <div className="mt-3">
          <div className="h-2 bg-[var(--color-border)] rounded-full overflow-hidden">
            <div 
              className="h-full bg-gradient-to-r from-emerald-400 to-emerald-500 rounded-full transition-all" 
              style={{ width: `${healthPct}%` }}
            />
          </div>
        </div>
      </StatCard>
      
      <GaugeCard label="CPU Global" value={avgCpu} color="var(--color-accent)"/>
      <GaugeCard label="Memoria Global" value={avgMem} color="var(--color-purple)"/>
    </div>
  );
}

function InfrastructureContent({ loading, agents, onSelectServer, onInstall }: {
  loading: boolean;
  agents: Agent[];
  onSelectServer: (agent: Agent) => void;
  onInstall: () => void;
}) {
  if (loading) {
    return <div className="text-center py-8 text-[var(--color-text-secondary)]">Cargando...</div>;
  }
  
  if (agents.length === 0) {
    return (
      <div className="text-center py-8 text-[var(--color-text-secondary)]">
        <p>No hay dispositivos</p>
        <button 
          onClick={onInstall}
          className="mt-4 px-4 py-2 bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-[var(--color-bg-primary)] text-[13px] font-medium rounded-lg transition-colors"
        >
          Instalar primer agente
        </button>
      </div>
    );
  }
  
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4 gap-3">
      {agents.map(agent => (
        <ServerCard
          key={agent.id}
          name={agent.name || agent.id}
          type={agent.type || "Server"}
          status={agent.status === "online" ? "online" : "offline"}
          cpu={Math.round(agent.cpu_usage || 0)}
          memory={Math.round(agent.memory_usage || 0)}
          lastSeen={agent.last_seen || "Desconocido"}
          onClick={() => onSelectServer(agent)}
        />
      ))}
    </div>
  );
}

function AnalysisCards({ onPrediction, onReport }: { onPrediction: () => void; onReport: () => void }) {
  return (
    <div className="grid grid-cols-1 md:grid-cols-2 gap-3">
      <article className="bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg p-4">
        <div className="flex items-center gap-2.5 mb-3">
          <div className="w-9 h-9 bg-gradient-to-br from-emerald-400 to-indigo-500 rounded-lg flex items-center justify-center text-white">
            <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <circle cx="12" cy="12" r="10"/><polyline points="12 6 12 12 16 14"/>
            </svg>
          </div>
          <div>
            <h3 className="text-[14px] font-semibold text-white">Chronos</h3>
          </div>
        </div>
        <p className="text-[13px] text-[var(--color-text-secondary)] leading-relaxed">
          Genera predicciones de series temporales.
        </p>
        <button 
          onClick={onPrediction}
          className="mt-3 w-full py-2 bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-[var(--color-bg-primary)] text-[13px] font-medium rounded-md transition-colors"
        >
          Ver Predicciones
        </button>
      </article>
      
      <article className="bg-[var(--color-bg-primary)] border border-[var(--color-border)] rounded-lg p-4">
        <div className="flex items-center gap-2.5 mb-3">
          <div className="w-9 h-9 bg-gradient-to-br from-indigo-400 to-purple-500 rounded-lg flex items-center justify-center text-white">
            <svg className="w-5 h-5" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
              <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z"/>
              <polyline points="14 2 14 8 20 8"/>
            </svg>
          </div>
          <div>
            <h3 className="text-[14px] font-semibold text-white">Generador Informes</h3>
          </div>
        </div>
        <p className="text-[13px] text-[var(--color-text-secondary)] leading-relaxed">
          Genera informes ejecutivos, tecnicos y de red.
        </p>
        <button 
          onClick={onReport}
          className="mt-3 w-full py-2 bg-[var(--color-accent)] hover:bg-[var(--color-accent-hover)] text-[var(--color-bg-primary)] text-[13px] font-medium rounded-md transition-colors"
        >
          Generar Informe
        </button>
      </article>
    </div>
  );
}

function AlertIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M10.29 3.86L1.82 18a2 2 0 0 0 1.71 3h16.94a2 2 0 0 0 1.71-3L13.71 3.86a2 2 0 0 0-3.42 0z"/>
      <line x1="12" y1="9" x2="12" y2="13"/>
      <line x1="12" y1="17" x2="12.01" y2="17"/>
    </svg>
  );
}

function InfraIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <rect x="2" y="3" width="20" height="14" rx="2" ry="2"/>
      <line x1="8" y1="21" x2="16" y2="21"/>
      <line x1="12" y1="17" x2="12" y2="21"/>
    </svg>
  );
}

function NetworkIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <circle cx="12" cy="12" r="10"/>
      <line x1="2" y1="12" x2="22" y2="12"/>
      <path d="M12 2a15.3 15.3 0 0 1 4 10 15.3 15.3 0 0 1-4 10 15.3 15.3 0 0 1-4-10 15.3 15.3 0 0 1 4-10z"/>
    </svg>
  );
}

function AnalysisIcon() {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M12 2a10 10 0 1 0 10 10H12V2z"/>
      <path d="M12 2a10 10 0 0 1 10 10"/>
    </svg>
  );
}
