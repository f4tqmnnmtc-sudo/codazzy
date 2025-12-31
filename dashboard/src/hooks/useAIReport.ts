import { useReducer, useCallback, useEffect } from "react";
import {
  getAvailableServers,
  collectReportData,
  generateReport,
  type ReportConfig,
  type MetricSummary,
} from "@/app/actions/reports";

const STORAGE_KEY = "ai-reports-dashboard";
const MAX_REPORTS = 10;

export interface Agent {
  id: string;
  name: string;
  status: string;
  cpu_usage: number;
  memory_usage: number;
}

export type { ReportConfig, MetricSummary };

export interface DebugPrompt {
  systemPrompt: string;
  userPrompt: string;
  model: string;
  timestamp: string;
}

export interface GeneratedReport {
  id: string;
  title: string;
  type: string;
  content: string;
  generatedAt: string;
  status: "generating" | "completed" | "error";
  servers: string[];
  anomaliesCount: number;
  predictionsCount: number;
  metricsData?: MetricSummary[];
  debugPrompt?: DebugPrompt;
}

export const REPORT_TYPES = [
  { id: "executive", name: "Ejecutivo", description: "Resumen de alto nivel", selectServers: true },
  { id: "technical", name: "Tecnico", description: "Analisis detallado con metricas", selectServers: true },
  { id: "network_performance", name: "Estado de Red", description: "Analisis global de enlaces", selectServers: false },
] as const;

interface State {
  servers: string[];
  agentData: Agent[];
  config: ReportConfig;
  isGenerating: boolean;
  generatedReports: GeneratedReport[];
  selectedReport: GeneratedReport | null;
  error: string | null;
  showHistory: boolean;
}

type Action =
  | { type: "SET_SERVERS"; servers: string[]; agents: Agent[] }
  | { type: "UPDATE_CONFIG"; updates: Partial<ReportConfig> }
  | { type: "TOGGLE_SERVER"; serverId: string }
  | { type: "SELECT_ALL_SERVERS" }
  | { type: "DESELECT_ALL_SERVERS" }
  | { type: "SET_GENERATING"; value: boolean }
  | { type: "ADD_REPORT"; report: GeneratedReport }
  | { type: "SELECT_REPORT"; report: GeneratedReport | null }
  | { type: "SET_ERROR"; error: string | null }
  | { type: "TOGGLE_HISTORY" }
  | { type: "LOAD_HISTORY"; reports: GeneratedReport[] };

const defaultConfig: ReportConfig = {
  title: "",
  type: "technical",
  servers: [],
  timeRange: "24h",
  includeAnomalies: true,
  includePredictions: true,
  includeRecommendations: true,
  language: "es",
  format: "markdown",
};

const initialState: State = {
  servers: [],
  agentData: [],
  config: defaultConfig,
  isGenerating: false,
  generatedReports: [],
  selectedReport: null,
  error: null,
  showHistory: false,
};

function reducer(state: State, action: Action): State {
  switch (action.type) {
    case "SET_SERVERS":
      return {
        ...state,
        servers: action.servers,
        agentData: action.agents,
        config: { ...state.config, servers: state.config.servers.length === 0 ? action.servers : state.config.servers },
      };
    case "UPDATE_CONFIG":
      return { ...state, config: { ...state.config, ...action.updates } };
    case "TOGGLE_SERVER": {
      const servers = state.config.servers.includes(action.serverId)
        ? state.config.servers.filter((s) => s !== action.serverId)
        : [...state.config.servers, action.serverId];
      return { ...state, config: { ...state.config, servers } };
    }
    case "SELECT_ALL_SERVERS":
      return { ...state, config: { ...state.config, servers: [...state.servers] } };
    case "DESELECT_ALL_SERVERS":
      return { ...state, config: { ...state.config, servers: [] } };
    case "SET_GENERATING":
      return { ...state, isGenerating: action.value };
    case "ADD_REPORT":
      return {
        ...state,
        generatedReports: [action.report, ...state.generatedReports].slice(0, MAX_REPORTS),
        selectedReport: action.report,
      };
    case "SELECT_REPORT":
      return { ...state, selectedReport: action.report };
    case "SET_ERROR":
      return { ...state, error: action.error };
    case "TOGGLE_HISTORY":
      return { ...state, showHistory: !state.showHistory };
    case "LOAD_HISTORY":
      return { ...state, generatedReports: action.reports };
    default:
      return state;
  }
}

export function useAIReport(initialAgents: Agent[] = []) {
  const [state, dispatch] = useReducer(reducer, initialState);

  useEffect(() => {
    try {
      const saved = localStorage.getItem(STORAGE_KEY);
      if (saved) dispatch({ type: "LOAD_HISTORY", reports: JSON.parse(saved) });
    } catch {}
  }, []);

  useEffect(() => {
    if (state.generatedReports.length > 0) {
      try {
        localStorage.setItem(STORAGE_KEY, JSON.stringify(state.generatedReports.slice(0, MAX_REPORTS)));
      } catch {}
    }
  }, [state.generatedReports]);

  const loadServers = useCallback(async () => {
    const { servers, agents } = await getAvailableServers(initialAgents);
    dispatch({ type: "SET_SERVERS", servers, agents });
  }, [initialAgents]);

  const handleGenerateReport = useCallback(async () => {
    const isNetwork = state.config.type === "network_performance";
    const serversToUse = isNetwork ? state.servers : state.config.servers;

    if (serversToUse.length === 0) {
      dispatch({ type: "SET_ERROR", error: isNetwork ? "No hay dispositivos de red" : "Selecciona al menos un servidor" });
      return;
    }

    dispatch({ type: "SET_GENERATING", value: true });
    dispatch({ type: "SET_ERROR", error: null });

    try {
      const title = isNetwork
        ? `Estado de Red - ${new Date().toLocaleDateString()}`
        : state.config.title || `Informe ${REPORT_TYPES.find((t) => t.id === state.config.type)?.name}`;

      const reportData = await collectReportData(state.agentData, { ...state.config, title, servers: serversToUse }, serversToUse);
      const result = await generateReport(state.config.type, { ...state.config, title, servers: serversToUse }, reportData);

      if (result.success && result.content) {
        const report: GeneratedReport = {
          id: `report-${Date.now()}`,
          title,
          type: REPORT_TYPES.find((t) => t.id === state.config.type)?.name || state.config.type,
          content: result.content,
          generatedAt: new Date().toISOString(),
          status: "completed",
          servers: serversToUse,
          anomaliesCount: reportData.anomalies.length,
          predictionsCount: reportData.predictions.length,
          metricsData: reportData.metrics,
          debugPrompt: result.debugPrompt,
        };
        dispatch({ type: "ADD_REPORT", report });
      } else {
        dispatch({ type: "SET_ERROR", error: result.error || "Error generando informe" });
      }
    } catch {
      dispatch({ type: "SET_ERROR", error: "Error de conexion" });
    } finally {
      dispatch({ type: "SET_GENERATING", value: false });
    }
  }, [state.config, state.servers, state.agentData]);

  const updateConfig = useCallback((updates: Partial<ReportConfig>) => dispatch({ type: "UPDATE_CONFIG", updates }), []);
  const toggleServer = useCallback((id: string) => dispatch({ type: "TOGGLE_SERVER", serverId: id }), []);
  const selectAllServers = useCallback(() => dispatch({ type: "SELECT_ALL_SERVERS" }), []);
  const deselectAllServers = useCallback(() => dispatch({ type: "DESELECT_ALL_SERVERS" }), []);
  const selectReport = useCallback((report: GeneratedReport | null) => dispatch({ type: "SELECT_REPORT", report }), []);
  const toggleHistory = useCallback(() => dispatch({ type: "TOGGLE_HISTORY" }), []);

  return {
    servers: state.servers,
    agentData: state.agentData,
    config: state.config,
    isGenerating: state.isGenerating,
    generatedReports: state.generatedReports,
    selectedReport: state.selectedReport,
    error: state.error,
    showHistory: state.showHistory,
    loadServers,
    generateReport: handleGenerateReport,
    updateConfig,
    toggleServer,
    selectAllServers,
    deselectAllServers,
    selectReport,
    toggleHistory,
  };
}

export function getMetricValue(report: GeneratedReport | null, metricType: string): number {
  if (!report?.metricsData) return 0;
  const metric = report.metricsData.find((m) => m.metricType === metricType);
  return metric?.avg || 0;
}
