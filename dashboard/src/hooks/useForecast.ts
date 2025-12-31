import { useReducer, useCallback, useMemo, useEffect, useRef } from "react";
import { chronosService, type ChronosForecastResponse } from "@/services/chronos.service";
import {
  influxDataService,
  type InfluxMetricPoint,
  type ServerInfo,
  type MetricType,
} from "@/services/influx-data.service";
import { savePredictions as savePredictionsAction } from "@/app/actions/server-panel";

export interface ChartDataPoint {
  timestamp: string;
  timeLabel: string;
  historical?: number;
  forecast_median?: number;
  forecast_low?: number;
  forecast_high?: number;
  confidence_10?: number;
  confidence_90?: number;
  isHistorical: boolean;
  isForecast: boolean;
}

export const HORIZON_OPTIONS = [
  { value: "2 hours", label: "2 horas", minPoints: 6 },
  { value: "4 hours", label: "4 horas", minPoints: 12 },
  { value: "8 hours", label: "8 horas", minPoints: 24 },
  { value: "12 hours", label: "12 horas", minPoints: 48 },
  { value: "1 day", label: "1 dia", minPoints: 7 },
  { value: "3 days", label: "3 dias", minPoints: 14 },
  { value: "7 days", label: "1 semana", minPoints: 30 },
  { value: "14 days", label: "2 semanas", minPoints: 60 },
  { value: "30 days", label: "1 mes", minPoints: 90 },
] as const;

export const TIME_RANGE_OPTIONS = [
  { value: "1h", label: "1 hora" },
  { value: "6h", label: "6 horas" },
  { value: "24h", label: "24 horas" },
  { value: "7d", label: "7 dias" },
  { value: "30d", label: "30 dias" },
] as const;

interface ForecastState {
  servers: ServerInfo[];
  metrics: MetricType[];
  selectedServer: string;
  selectedMetrics: string[];
  activeMetric: string;
  historicalData: InfluxMetricPoint[];
  forecastData: ChronosForecastResponse | null;
  chronosHealth: { status: string; model_loaded: boolean } | null;
  timeRange: string;
  forecastHorizon: string;
  aggregationMethod: "mean" | "median" | "max" | "min";
  loading: boolean;
  generating: boolean;
  error: string | null;
}

type Action =
  | { type: "INIT"; servers: ServerInfo[]; metrics: MetricType[]; health: ForecastState["chronosHealth"] }
  | { type: "SELECT_SERVER"; server: string; metrics: string[]; active: string }
  | { type: "SET_ACTIVE_METRIC"; metric: string }
  | { type: "TOGGLE_METRIC"; metric: string }
  | { type: "SET_HISTORICAL"; data: InfluxMetricPoint[] }
  | { type: "SET_FORECAST"; data: ChronosForecastResponse }
  | { type: "SET_TIME_RANGE"; value: string }
  | { type: "SET_HORIZON"; value: string }
  | { type: "SET_AGGREGATION"; value: ForecastState["aggregationMethod"] }
  | { type: "SET_LOADING"; value: boolean }
  | { type: "SET_GENERATING"; value: boolean }
  | { type: "SET_ERROR"; error: string | null }
  | { type: "CLEAR_FORECAST" };

const initialState: ForecastState = {
  servers: [],
  metrics: [],
  selectedServer: "",
  selectedMetrics: [],
  activeMetric: "",
  historicalData: [],
  forecastData: null,
  chronosHealth: null,
  timeRange: "7d",
  forecastHorizon: "12 hours",
  aggregationMethod: "median",
  loading: true,
  generating: false,
  error: null,
};

function reducer(state: ForecastState, action: Action): ForecastState {
  switch (action.type) {
    case "INIT":
      return { ...state, servers: action.servers, metrics: action.metrics, chronosHealth: action.health, loading: false };
    case "SELECT_SERVER":
      return { ...state, selectedServer: action.server, selectedMetrics: action.metrics, activeMetric: action.active, forecastData: null };
    case "SET_ACTIVE_METRIC":
      return { ...state, activeMetric: action.metric, forecastData: null };
    case "TOGGLE_METRIC": {
      const has = state.selectedMetrics.includes(action.metric);
      const next = has
        ? state.selectedMetrics.filter((m) => m !== action.metric)
        : state.selectedMetrics.length >= 4
          ? [...state.selectedMetrics.slice(1), action.metric]
          : [...state.selectedMetrics, action.metric];
      const active = has && state.activeMetric === action.metric && next.length > 0 ? next[0] : state.activeMetric;
      return { ...state, selectedMetrics: next, activeMetric: active };
    }
    case "SET_HISTORICAL":
      return { ...state, historicalData: action.data, loading: false };
    case "SET_FORECAST":
      return { ...state, forecastData: action.data, generating: false };
    case "SET_TIME_RANGE":
      return { ...state, timeRange: action.value };
    case "SET_HORIZON":
      return { ...state, forecastHorizon: action.value };
    case "SET_AGGREGATION":
      return { ...state, aggregationMethod: action.value };
    case "SET_LOADING":
      return { ...state, loading: action.value };
    case "SET_GENERATING":
      return { ...state, generating: action.value };
    case "SET_ERROR":
      return { ...state, error: action.error, loading: false, generating: false };
    case "CLEAR_FORECAST":
      return { ...state, forecastData: null };
    default:
      return state;
  }
}

const CHRONOS_MAX_CONTEXT = 1024;

export function useForecast() {
  const [state, dispatch] = useReducer(reducer, initialState);
  const initRef = useRef(false);

  const getServerInfo = useCallback(
    (key: string): ServerInfo | undefined => {
      const parts = key.split("_");
      const type = parts.pop();
      const id = parts.join("_");
      return state.servers.find((s) => s.server_id === id && s.server_type === type);
    },
    [state.servers]
  );

  const getMetricInfo = useCallback(
    (type: string): MetricType | undefined => state.metrics.find((m) => m.metric_type === type),
    [state.metrics]
  );

  useEffect(() => {
    if (initRef.current) return;
    initRef.current = true;

    (async () => {
      try {
        const [servers, metrics] = await Promise.all([
          influxDataService.getAvailableServers(),
          influxDataService.getAvailableMetricTypes(),
        ]);

        let health: ForecastState["chronosHealth"] = null;
        try {
          health = await chronosService.getHealth();
        } catch {
          health = { status: "unavailable", model_loaded: false };
        }

        dispatch({ type: "INIT", servers, metrics, health });

        const firstAgent = servers.find((s) => s.server_type === "agent");
        if (firstAgent) {
          const key = `${firstAgent.server_id}_${firstAgent.server_type}`;
          dispatch({ type: "SELECT_SERVER", server: key, metrics: ["cpu", "memory", "disk"], active: "cpu" });
        } else if (servers.length > 0) {
          const key = `${servers[0].server_id}_${servers[0].server_type}`;
          dispatch({ type: "SELECT_SERVER", server: key, metrics: [], active: "" });
        }
      } catch (err) {
        dispatch({ type: "SET_ERROR", error: err instanceof Error ? err.message : "Error loading data" });
      }
    })();
  }, []);

  const loadHistoricalData = useCallback(async () => {
    if (!state.selectedServer || !state.activeMetric) return;

    dispatch({ type: "SET_LOADING", value: true });
    dispatch({ type: "SET_ERROR", error: null });
    dispatch({ type: "CLEAR_FORECAST" });

    try {
      const info = getServerInfo(state.selectedServer);
      const data = await influxDataService.getHistoricalData(
        info?.server_id || "",
        state.activeMetric,
        state.timeRange,
        1024
      );

      if (data.length === 0) {
        dispatch({ type: "SET_ERROR", error: "No hay datos historicos disponibles" });
        dispatch({ type: "SET_HISTORICAL", data: [] });
      } else {
        dispatch({ type: "SET_HISTORICAL", data });
      }
    } catch (err) {
      dispatch({ type: "SET_ERROR", error: err instanceof Error ? err.message : "Error loading historical data" });
      dispatch({ type: "SET_HISTORICAL", data: [] });
    }
  }, [state.selectedServer, state.activeMetric, state.timeRange, getServerInfo]);

  const generateForecast = useCallback(async () => {
    const horizon = HORIZON_OPTIONS.find((h) => h.value === state.forecastHorizon);
    const minPoints = horizon?.minPoints || 6;

    if (state.historicalData.length < minPoints) {
      dispatch({
        type: "SET_ERROR",
        error: `Datos insuficientes. Se necesitan ${minPoints} puntos (hay ${state.historicalData.length}).`,
      });
      return;
    }

    dispatch({ type: "SET_GENERATING", value: true });
    dispatch({ type: "SET_ERROR", error: null });

    try {
      const serverInfo = getServerInfo(state.selectedServer);
      const metricInfo = getMetricInfo(state.activeMetric);
      const serverId = serverInfo?.server_id || "";

      const dataForChronos =
        state.historicalData.length > CHRONOS_MAX_CONTEXT
          ? state.historicalData.slice(-CHRONOS_MAX_CONTEXT)
          : state.historicalData;

      const chronosMetrics = influxDataService.convertToChronosFormat(
        dataForChronos,
        `${serverInfo?.server_name}_${metricInfo?.metric_name}`,
        serverId
      );

      const useLongTerm = ["day", "days", "week", "month"].some((p) =>
        state.forecastHorizon.toLowerCase().includes(p)
      );

      const request = {
        metrics: { ...chronosMetrics, metric_type: state.activeMetric, unit: metricInfo?.unit || "%" },
        period_type: useLongTerm ? ("week" as const) : ("day" as const),
        aggregation_method: state.aggregationMethod,
        prediction_horizon: state.forecastHorizon,
        num_samples: 500,
        confidence_levels: [0.1, 0.2, 0.5, 0.8, 0.9],
        include_analysis: true,
      };

      const forecast = useLongTerm
        ? await chronosService.getWeeklyForecast(request)
        : await chronosService.getDailyForecast(request);

      if (forecast.forecast_timestamps?.length) {
        const firstTs = new Date(forecast.forecast_timestamps[0]).getTime();
        const lastTs = new Date(forecast.forecast_timestamps[forecast.forecast_timestamps.length - 1]).getTime();
        const actualDays = (lastTs - firstTs) / (1000 * 60 * 60 * 24);
        const requestedDays = parseHorizonToDays(state.forecastHorizon);
        
        if (actualDays < requestedDays * 0.7) {
          console.warn(`Chronos devolvio ${actualDays.toFixed(1)} dias en lugar de ${requestedDays} solicitados`);
        }
      }

      dispatch({ type: "SET_FORECAST", data: forecast });

      const timestamps = forecast.forecast_timestamps || [];
      const median = forecast.forecast_values["0.5"] || forecast.forecast_values["50"] || [];
      const low = forecast.forecast_values["0.2"] || forecast.forecast_values["20"] || [];
      const high = forecast.forecast_values["0.8"] || forecast.forecast_values["80"] || [];

      if (timestamps.length && median.length) {
        const predictions = timestamps.map((ts, i) => ({
          timestamp: Math.floor(new Date(ts).getTime() / 1000),
          value: median[i] || 0,
          lower_bound: low[i] || median[i] * 0.8,
          upper_bound: high[i] || median[i] * 1.2,
          confidence: 0.9,
        }));
        savePredictionsAction(serverId, state.activeMetric, predictions).catch(() => {});
      }
    } catch (err) {
      dispatch({ type: "SET_ERROR", error: err instanceof Error ? err.message : "Error generando prediccion" });
    }
  }, [state.historicalData, state.selectedServer, state.activeMetric, state.forecastHorizon, state.aggregationMethod, getServerInfo, getMetricInfo]);

  const chartData = useMemo((): ChartDataPoint[] => {
    const data: ChartDataPoint[] = [];
    const MAX_CHART_POINTS = 300;
    
    const histStep = Math.max(1, Math.floor(state.historicalData.length / 150));
    const sampled = state.historicalData.filter((_, i) => i % histStep === 0 || i === state.historicalData.length - 1);

    sampled.forEach((point) => {
      data.push({
        timestamp: point.timestamp,
        timeLabel: new Date(point.timestamp).toLocaleString("es-ES", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }),
        historical: point.value,
        isHistorical: true,
        isForecast: false,
      });
    });

    if (state.forecastData?.forecast_timestamps?.length) {
      const lastHist = sampled[sampled.length - 1];
      const lastTime = lastHist ? new Date(lastHist.timestamp).getTime() : 0;

      const forecastTimestamps = state.forecastData.forecast_timestamps.filter(ts => new Date(ts).getTime() > lastTime);
      const forecastStep = Math.max(1, Math.floor(forecastTimestamps.length / (MAX_CHART_POINTS - sampled.length)));

      forecastTimestamps.forEach((ts, idx) => {
        if (idx % forecastStep !== 0 && idx !== forecastTimestamps.length - 1) return;
        
        const originalIdx = state.forecastData!.forecast_timestamps.indexOf(ts);
        const isFirst = !data.some((d) => d.isForecast);

        data.push({
          timestamp: ts,
          timeLabel: new Date(ts).toLocaleString("es-ES", { day: "2-digit", month: "2-digit", hour: "2-digit", minute: "2-digit" }),
          historical: isFirst && lastHist ? lastHist.value : undefined,
          forecast_median: state.forecastData!.forecast_values["0.5"]?.[originalIdx] || state.forecastData!.forecast_values["50"]?.[originalIdx],
          forecast_low: state.forecastData!.forecast_values["0.2"]?.[originalIdx] || state.forecastData!.forecast_values["20"]?.[originalIdx],
          forecast_high: state.forecastData!.forecast_values["0.8"]?.[originalIdx] || state.forecastData!.forecast_values["80"]?.[originalIdx],
          confidence_10: state.forecastData!.forecast_values["0.1"]?.[originalIdx] || state.forecastData!.forecast_values["10"]?.[originalIdx],
          confidence_90: state.forecastData!.forecast_values["0.9"]?.[originalIdx] || state.forecastData!.forecast_values["90"]?.[originalIdx],
          isHistorical: isFirst,
          isForecast: true,
        });
      });
    }

    return data.sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
  }, [state.historicalData, state.forecastData]);

  const selectServer = useCallback((key: string) => {
    const parts = key.split("_");
    const type = parts.pop();
    const metrics = type === "agent" ? ["cpu", "memory", "disk"] : [];
    dispatch({ type: "SELECT_SERVER", server: key, metrics, active: metrics[0] || "" });
  }, []);

  const setActiveMetric = useCallback((metric: string) => {
    dispatch({ type: "SET_ACTIVE_METRIC", metric });
  }, []);

  const toggleMetric = useCallback((metric: string) => {
    dispatch({ type: "TOGGLE_METRIC", metric });
  }, []);

  const setTimeRange = useCallback((value: string) => {
    dispatch({ type: "SET_TIME_RANGE", value });
  }, []);

  const setForecastHorizon = useCallback((value: string) => {
    dispatch({ type: "SET_HORIZON", value });
  }, []);

  const setAggregationMethod = useCallback((value: ForecastState["aggregationMethod"]) => {
    dispatch({ type: "SET_AGGREGATION", value });
  }, []);

  const exportToCSV = useCallback(() => {
    if (!state.forecastData) return;
    const rows = [["Timestamp", "Historical", "Forecast_Median", "Forecast_Low", "Forecast_High"]];
    chartData.forEach((p) => rows.push([p.timestamp, p.historical?.toString() || "", p.forecast_median?.toString() || "", p.forecast_low?.toString() || "", p.forecast_high?.toString() || ""]));
    downloadFile(rows.map((r) => r.join(",")).join("\n"), `forecast_${state.selectedServer}_${state.activeMetric}.csv`, "text/csv");
  }, [state.forecastData, chartData, state.selectedServer, state.activeMetric]);

  const exportToJSON = useCallback(() => {
    if (!state.forecastData) return;
    const data = {
      metadata: { server: state.selectedServer, metric: state.activeMetric, timestamp: new Date().toISOString(), time_range: state.timeRange, forecast_horizon: state.forecastHorizon },
      historical_data: state.historicalData,
      forecast_data: state.forecastData,
    };
    downloadFile(JSON.stringify(data, null, 2), `forecast_${state.selectedServer}_${state.activeMetric}.json`, "application/json");
  }, [state.forecastData, state.historicalData, state.selectedServer, state.activeMetric, state.timeRange, state.forecastHorizon]);

  return {
    ...state,
    chartData,
    loadHistoricalData,
    generateForecast,
    selectServer,
    setActiveMetric,
    toggleMetric,
    setTimeRange,
    setForecastHorizon,
    setAggregationMethod,
    exportToCSV,
    exportToJSON,
    getServerInfo,
    getMetricInfo,
    canGenerate: !state.generating && state.chronosHealth?.model_loaded && state.historicalData.length >= 10,
    isChronosAvailable: state.chronosHealth?.model_loaded ?? false,
  };
}

function downloadFile(content: string, filename: string, type: string) {
  const blob = new Blob([content], { type });
  const url = URL.createObjectURL(blob);
  const a = document.createElement("a");
  a.href = url;
  a.download = filename;
  a.click();
  URL.revokeObjectURL(url);
}

function parseHorizonToDays(horizon: string): number {
  const match = horizon.match(/(\d+)\s*(hour|hours|day|days|week|weeks|month|months)/i);
  if (!match) return 1;
  const num = parseInt(match[1], 10);
  const unit = match[2].toLowerCase();
  if (unit.startsWith("hour")) return num / 24;
  if (unit.startsWith("day")) return num;
  if (unit.startsWith("week")) return num * 7;
  if (unit.startsWith("month")) return num * 30;
  return num;
}

