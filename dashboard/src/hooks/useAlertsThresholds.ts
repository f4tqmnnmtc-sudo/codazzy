import { useReducer, useCallback, useRef, useMemo } from "react";
import { chronosService } from "@/services/chronos.service";
import { gatewayService } from "@/services/gateway.service";
import {
  getThresholds,
  getDiskValue,
  analyzeServerThresholds,
  getMetricHistory,
} from "@/app/actions/alerts";
import type { Agent } from "@/types/gateway";
import type {
  DeviceThresholds,
  ActiveAlert,
  PredictedAnomaly,
  MetricValues,
} from "@/components/sections/alerts/types";

interface AlertsState {
  thresholds: DeviceThresholds[];
  diskValues: Record<string, number | null>;
  predictions: PredictedAnomaly[];
  cachedPredictions: PredictedAnomaly[];
  loading: boolean;
  analyzing: string | null;
  predicting: boolean;
  error: string | null;
  predictionError: string | null;
  chronosOnline: boolean | null;
  expanded: Set<string>;
  showPredictions: boolean;
  initialized: boolean;
}

type AlertsAction =
  | { type: "INIT_DATA"; thresholds: DeviceThresholds[]; diskValues: Record<string, number | null>; chronosOnline: boolean | null; cachedPredictions: PredictedAnomaly[] }
  | { type: "SET_THRESHOLDS"; payload: DeviceThresholds[] }
  | { type: "SET_PREDICTIONS"; payload: PredictedAnomaly[] }
  | { type: "SET_ANALYZING"; payload: string | null }
  | { type: "SET_PREDICTING"; payload: boolean }
  | { type: "SET_ERROR"; payload: string | null }
  | { type: "SET_PREDICTION_ERROR"; payload: string | null }
  | { type: "TOGGLE_EXPANDED"; payload: string }
  | { type: "SET_SHOW_PREDICTIONS"; payload: boolean };

const initialState: AlertsState = {
  thresholds: [],
  diskValues: {},
  predictions: [],
  cachedPredictions: [],
  loading: true,
  analyzing: null,
  predicting: false,
  error: null,
  predictionError: null,
  chronosOnline: null,
  expanded: new Set(),
  showPredictions: false,
  initialized: false,
};

function alertsReducer(state: AlertsState, action: AlertsAction): AlertsState {
  switch (action.type) {
    case "INIT_DATA":
      return {
        ...state,
        thresholds: action.thresholds,
        diskValues: action.diskValues,
        chronosOnline: action.chronosOnline,
        cachedPredictions: action.cachedPredictions,
        loading: false,
        initialized: true,
      };
    case "SET_THRESHOLDS":
      return { ...state, thresholds: action.payload };
    case "SET_PREDICTIONS":
      return { ...state, predictions: action.payload };
    case "SET_ANALYZING":
      return { ...state, analyzing: action.payload };
    case "SET_PREDICTING":
      return { ...state, predicting: action.payload };
    case "SET_ERROR":
      return { ...state, error: action.payload };
    case "SET_PREDICTION_ERROR":
      return { ...state, predictionError: action.payload };
    case "TOGGLE_EXPANDED": {
      const next = new Set(state.expanded);
      next.has(action.payload) ? next.delete(action.payload) : next.add(action.payload);
      return { ...state, expanded: next };
    }
    case "SET_SHOW_PREDICTIONS":
      return { ...state, showPredictions: action.payload };
    default:
      return state;
  }
}

function computeAlerts(
  thresholds: DeviceThresholds[],
  agents: Agent[],
  metrics: Record<string, MetricValues>
): ActiveAlert[] {
  if (thresholds.length === 0 || agents.length === 0) return [];

  const alerts: ActiveAlert[] = [];

  for (const agent of agents) {
    const deviceTh = thresholds.find((t) => t.device_id === agent.id);
    const m = metrics[agent.id];
    if (!deviceTh || !m) continue;

    for (const th of deviceTh.thresholds) {
      if (!th.enabled) continue;
      const name = th.metric_name.toLowerCase();
      let val: number | null = null;
      if (name.includes("cpu")) val = m.cpu;
      else if (name.includes("mem")) val = m.mem;
      else if (name.includes("disk")) val = m.disk;
      if (val === null) continue;

      const above = th.comparison === "gt" || th.comparison === "gte";
      let severity: "warning" | "critical" | null = null;
      let limit: number | null = null;

      if (th.critical !== null && (above ? val >= th.critical : val <= th.critical)) {
        severity = "critical";
        limit = th.critical;
      } else if (th.warning !== null && (above ? val >= th.warning : val <= th.warning)) {
        severity = "warning";
        limit = th.warning;
      }

      if (severity && limit !== null) {
        alerts.push({
          id: `${agent.id}-${th.metric_name}`,
          server_id: agent.id,
          server_name: agent.name || agent.id,
          metric_name: th.metric_name,
          display_name: th.display_name || th.metric_name,
          current_value: val,
          threshold_value: limit,
          severity,
          comparison: th.comparison,
          unit: th.unit || "%",
          detected_at: new Date().toISOString(),
        });
      }
    }
  }

  return alerts;
}

export function useAlertsThresholds(agents: Agent[]) {
  const [state, dispatch] = useReducer(alertsReducer, initialState);
  const loadingRef = useRef(false);
  const initializedAgentsRef = useRef("");

  const agentIds = useMemo(() => agents.map((a) => a.id).sort().join(","), [agents]);

  const metrics = useMemo(() => {
    const m: Record<string, MetricValues> = {};
    for (const a of agents) {
      m[a.id] = { cpu: a.cpu_usage, mem: a.memory_usage, disk: state.diskValues[a.id] ?? null };
    }
    return m;
  }, [agents, state.diskValues]);

  const unconfigured = useMemo(
    () => agents.filter((a) => !state.thresholds.some((t) => t.device_id === a.id)),
    [agents, state.thresholds]
  );

  const activeAlerts = useMemo(
    () => computeAlerts(state.thresholds, agents, metrics),
    [state.thresholds, agents, metrics]
  );

  const shouldLoad = agentIds !== initializedAgentsRef.current && agents.length > 0;

  const loadInitialData = useCallback(async (force = false) => {
    if (loadingRef.current) return;
    if (!force && initializedAgentsRef.current === agentIds) return;
    if (agents.length === 0) {
      dispatch({ type: "INIT_DATA", thresholds: [], diskValues: {}, chronosOnline: null, cachedPredictions: [] });
      return;
    }

    loadingRef.current = true;
    initializedAgentsRef.current = agentIds;

    const [diskResults, thresholdResults, healthResult, cachedResult] = await Promise.all([
      Promise.all(agents.map(async (agent) => ({ id: agent.id, value: await getDiskValue(agent.id) }))),
      Promise.all(agents.map((agent) => getThresholds(agent.id))),
      chronosService.getHealth().catch(() => ({ status: "unavailable", model_loaded: false })),
      gatewayService.getCachedPredictions(),
    ]);

    const diskValues: Record<string, number | null> = {};
    for (const r of diskResults) diskValues[r.id] = r.value;

    const validThresholds = thresholdResults.filter((t): t is DeviceThresholds => t !== null);
    const chronosOnline = healthResult.status === "healthy" || healthResult.model_loaded;

    const cachedPredictions: PredictedAnomaly[] = cachedResult.predictions.map((p) => ({
      id: p.id,
      server_id: p.device_id,
      server_name: p.device_name,
      metric_name: p.metric_name,
      display_name: p.display_name || p.metric_name,
      predicted_value: p.predicted_value,
      threshold_value: p.threshold_value,
      threshold_type: p.threshold_type as "warning" | "critical",
      predicted_at: p.predicted_at,
      confidence: p.confidence,
      current_value: p.current_value,
      trend: p.trend as "increasing" | "decreasing" | "stable",
      hours_until: p.hours_until,
    }));

    dispatch({ type: "INIT_DATA", thresholds: validThresholds, diskValues, chronosOnline, cachedPredictions });
    loadingRef.current = false;
  }, [agents, agentIds]);

  const reloadThresholds = useCallback(async () => {
    const results = await Promise.all(agents.map((agent) => getThresholds(agent.id)));
    dispatch({ type: "SET_THRESHOLDS", payload: results.filter((t): t is DeviceThresholds => t !== null) });
  }, [agents]);

  const analyzeServer = useCallback(
    async (agent: Agent) => {
      dispatch({ type: "SET_ANALYZING", payload: agent.id });
      dispatch({ type: "SET_ERROR", payload: null });

      const m = metrics[agent.id];
      const result = await analyzeServerThresholds(
        agent.id,
        agent.name || agent.id,
        m ? { cpu_percent: m.cpu, memory_percent: m.mem, disk_percent: m.disk } : undefined
      );

      if (result.success) {
        await reloadThresholds();
        dispatch({ type: "TOGGLE_EXPANDED", payload: agent.id });
      } else {
        dispatch({ type: "SET_ERROR", payload: result.error || "Error al analizar" });
      }

      dispatch({ type: "SET_ANALYZING", payload: null });
    },
    [metrics, reloadThresholds]
  );

  const runPrediction = useCallback(async () => {
    if (!state.thresholds.length || !agents.length) {
      dispatch({ type: "SET_PREDICTION_ERROR", payload: "No hay umbrales configurados" });
      return;
    }

    dispatch({ type: "SET_PREDICTING", payload: true });
    dispatch({ type: "SET_PREDICTION_ERROR", payload: null });

    const results: PredictedAnomaly[] = [];

    try {
      for (const device of state.thresholds) {
        const agent = agents.find((a) => a.id === device.device_id);
        if (!agent) continue;

        for (const th of device.thresholds) {
          if (!th.enabled) continue;
          const name = th.metric_name.toLowerCase();
          let component = "";
          let val: number | null = null;

          if (name.includes("cpu")) {
            component = "cpu_percent";
            val = agent.cpu_usage;
          } else if (name.includes("mem")) {
            component = "memory_percent";
            val = agent.memory_usage;
          } else if (name.includes("disk")) {
            component = "fs_root_usage_percent";
            val = metrics[agent.id]?.disk ?? null;
          } else continue;

          const above = th.comparison === "gt" || th.comparison === "gte";
          const thresholdVal = th.warning ?? th.critical;
          if (thresholdVal === null) continue;

          const currentVal = val ?? 0;
          const alreadyBreached = above ? currentVal >= thresholdVal : currentVal <= thresholdVal;
          const thresholdType: "warning" | "critical" =
            th.critical !== null && (above ? currentVal >= th.critical : currentVal <= th.critical) ? "critical" : "warning";

          const history = await getMetricHistory(device.device_id, component);

          if (history.length >= 10) {
            try {
              const historyDurationMs =
                history.length > 1
                  ? new Date(history[history.length - 1].timestamp).getTime() - new Date(history[0].timestamp).getTime()
                  : 0;
              const historyHours = historyDurationMs / 3600000;
              const useWeekly = historyHours > 48;
              const maxPredictionHours = Math.min(useWeekly ? 24 : 4, Math.floor(historyHours / 4));
              const predictionHorizon = `${Math.max(1, maxPredictionHours)} hours`;

              const forecastRequest = {
                metrics: {
                  series_name: `${device.device_id}_${component}`,
                  server_id: device.device_id,
                  metric_type: component,
                  unit: "%",
                  data_points: history,
                },
                period_type: useWeekly ? ("week" as const) : ("day" as const),
                prediction_horizon: predictionHorizon,
                num_samples: 200,
                confidence_levels: [0.5, 0.9],
                include_analysis: true,
              };

              const forecast = useWeekly
                ? await chronosService.getWeeklyForecast(forecastRequest)
                : await chronosService.getDailyForecast(forecastRequest);

              const median = forecast.forecast_values["0.5"] || forecast.forecast_values["50"] || [];
              const times = forecast.forecast_timestamps || [];
              const historyVal = history[history.length - 1]?.value || currentVal;
              const maxPred = Math.max(...median, historyVal);
              const trend =
                forecast.analysis?.trend === "increasing"
                  ? "increasing"
                  : forecast.analysis?.trend === "decreasing"
                    ? "decreasing"
                    : "stable";
              const breachIdx = median.findIndex((p) => (above ? p >= thresholdVal : p <= thresholdVal));
              const willBreach = breachIdx !== -1;

              let hours_until: number;
              let predicted_value: number;
              let predicted_at: string;

              if (willBreach) {
                predicted_at = times[breachIdx] || new Date().toISOString();
                hours_until = Math.max(0, (new Date(predicted_at).getTime() - Date.now()) / 3600000);
                predicted_value = median[breachIdx];
                if (hours_until < 0.017 && alreadyBreached) hours_until = 0;
              } else if (alreadyBreached) {
                hours_until = 0;
                predicted_value = currentVal;
                predicted_at = new Date().toISOString();
              } else {
                hours_until = -1;
                predicted_value = maxPred;
                predicted_at = times[times.length - 1] || new Date().toISOString();
              }

              results.push({
                id: `${device.device_id}-${th.metric_name}`,
                server_id: device.device_id,
                server_name: device.device_name,
                metric_name: th.metric_name,
                display_name: th.display_name || th.metric_name,
                predicted_value: Math.round(predicted_value * 10) / 10,
                threshold_value: thresholdVal,
                threshold_type: thresholdType,
                predicted_at,
                confidence: Math.round((forecast.analysis?.confidence_score || 0.7) * 100),
                current_value: Math.round(historyVal * 10) / 10,
                trend,
                hours_until: Math.round(hours_until * 10) / 10,
              });
              continue;
            } catch {
              // Fall through
            }
          }

          if (alreadyBreached) {
            results.push({
              id: `${device.device_id}-${th.metric_name}`,
              server_id: device.device_id,
              server_name: device.device_name,
              metric_name: th.metric_name,
              display_name: th.display_name || th.metric_name,
              predicted_value: Math.round(currentVal * 10) / 10,
              threshold_value: thresholdVal,
              threshold_type: thresholdType,
              predicted_at: new Date().toISOString(),
              confidence: 100,
              current_value: Math.round(currentVal * 10) / 10,
              trend: "stable",
              hours_until: 0,
            });
          }
        }
      }

      results.sort((a, b) => a.hours_until - b.hours_until);
      dispatch({ type: "SET_PREDICTIONS", payload: results });
      dispatch({ type: "SET_SHOW_PREDICTIONS", payload: true });
    } catch {
      dispatch({ type: "SET_PREDICTION_ERROR", payload: "Error de conexion" });
    } finally {
      dispatch({ type: "SET_PREDICTING", payload: false });
    }
  }, [state.thresholds, agents, metrics]);

  const toggleExpanded = useCallback((id: string) => {
    dispatch({ type: "TOGGLE_EXPANDED", payload: id });
  }, []);

  const closePredictions = useCallback(() => {
    dispatch({ type: "SET_SHOW_PREDICTIONS", payload: false });
  }, []);

  return {
    thresholds: state.thresholds,
    activeAlerts,
    predictions: state.predictions,
    cachedPredictions: state.cachedPredictions,
    loading: state.loading,
    analyzing: state.analyzing,
    predicting: state.predicting,
    error: state.error,
    predictionError: state.predictionError,
    chronosOnline: state.chronosOnline,
    expanded: state.expanded,
    showPredictions: state.showPredictions,
    initialized: state.initialized,
    metrics,
    unconfigured,
    agentIds,
    shouldLoad,
    loadInitialData,
    analyzeServer,
    runPrediction,
    toggleExpanded,
    closePredictions,
  };
}
