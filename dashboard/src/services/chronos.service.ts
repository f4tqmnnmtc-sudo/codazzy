import {
  getChronosHealth,
  getDailyForecast as getDailyForecastAction,
  getWeeklyForecast as getWeeklyForecastAction,
  type ActionResult,
} from "@/app/actions/chronos";

export interface ChronosDataPoint {
  timestamp: string;
  value: number;
}

export interface ChronosMetrics {
  series_name: string;
  server_id?: string;
  metric_type?: string;
  unit?: string;
  data_points: ChronosDataPoint[];
}

export interface ChronosForecastRequest {
  metrics: ChronosMetrics;
  period_type: "day" | "week";
  aggregation_method?: "mean" | "median" | "max" | "min";
  prediction_horizon?: string;
  num_samples?: number;
  confidence_levels?: number[];
  include_analysis?: boolean;
}

export interface ChronosAggregation {
  original_points: number;
  aggregated_points: number;
  aggregation_ratio: number;
  original_frequency: string;
  target_frequency: string;
  aggregation_method: string;
  description: string;
  time_span: {
    start: string;
    end: string;
    duration_hours: number;
  };
}

export interface ChronosHistoricalData {
  timestamps: string[];
  values: number[];
  count: number;
  duration: string;
  resolution?: string;
}

export interface ChronosPatternsDetected {
  daily_cycle?: boolean;
  weekly_cycle?: boolean;
  workday_pattern?: boolean;
  weekend_pattern?: boolean;
  peak_hours?: number[];
  peak_days?: string[];
  low_hours?: number[];
  low_days?: string[];
  average_value: number;
  volatility: number;
  trend: "stable" | "increasing" | "decreasing";
}

export interface ChronosAnalysis {
  trend: "stable" | "increasing" | "decreasing";
  stability: "high" | "medium" | "low";
  confidence_score: number;
  anomalies_detected: number;
  prediction_stability: number;
  historical_stats?: {
    mean: number;
    std: number;
    min: number;
    max: number;
    median: number;
    q25: number;
    q75: number;
  };
  prediction_stats?: {
    mean: number;
    std: number;
    min: number;
    max: number;
  };
  trend_analysis?: {
    historical_trend: number;
    prediction_trend: number;
    trend_interpretation: string;
    trend_consistency: string;
  };
  quality_metrics?: {
    mean_change_percent: number;
    volatility_change_percent: number;
    prediction_stability: number;
  };
}

export interface ChronosModelInfo {
  model_name: string;
  device: string;
  torch_dtype: string;
  context_length: number;
  prediction_length: number;
  num_samples: number;
}

export interface ChronosForecastResponse {
  series_name: string;
  server_id?: string;
  period_type: "day" | "week";
  aggregation: ChronosAggregation;
  historical_data: ChronosHistoricalData;
  forecast_values: Record<string, number[]>;
  forecast_timestamps: string[];
  patterns_detected: ChronosPatternsDetected;
  analysis?: ChronosAnalysis;
  model_info: ChronosModelInfo;
  processing_time: number;
  timestamp: string;
}

export interface ChronosHealthResponse {
  status: string;
  gpu_available: boolean;
  model_loaded: boolean;
  memory_usage?: {
    gpu_memory_used: number;
    gpu_memory_total: number;
  };
}

function downsample(points: ChronosDataPoint[], target: number): ChronosDataPoint[] {
  if (points.length <= target) return points;

  const sorted = [...points].sort((a, b) => new Date(a.timestamp).getTime() - new Date(b.timestamp).getTime());
  const windowSize = Math.ceil(sorted.length / target);
  const result: ChronosDataPoint[] = [];

  for (let i = 0; i < sorted.length; i += windowSize) {
    const window = sorted.slice(i, Math.min(i + windowSize, sorted.length));
    if (window.length > 0) {
      const avg = window.reduce((sum, p) => sum + p.value, 0) / window.length;
      result.push({
        timestamp: window[window.length - 1].timestamp,
        value: Math.round(avg * 100) / 100,
      });
    }
  }

  return result;
}

function unwrapResult<T>(result: ActionResult<T>): T {
  if (!result.success) {
    throw new Error(result.error);
  }
  return result.data;
}

export const chronosService = {
  async getDailyForecast(request: ChronosForecastRequest): Promise<ChronosForecastResponse> {
    // Limitar a 512 puntos para predicciones más rápidas (era 1440)
    const MAX_POINTS = 512;
    if (request.metrics.data_points.length > MAX_POINTS) {
      request.metrics.data_points = downsample(request.metrics.data_points, MAX_POINTS);
    }
    const result = await getDailyForecastAction(request);
    return unwrapResult(result);
  },

  async getWeeklyForecast(request: ChronosForecastRequest): Promise<ChronosForecastResponse> {
    // Limitar a 1024 puntos para predicciones semanales (era 10080)
    const MAX_POINTS = 1024;
    if (request.metrics.data_points.length > MAX_POINTS) {
      request.metrics.data_points = downsample(request.metrics.data_points, MAX_POINTS);
    }
    const result = await getWeeklyForecastAction(request);
    return unwrapResult(result);
  },

  async getHealth(): Promise<ChronosHealthResponse> {
    return getChronosHealth();
  },
};
