"use server";

import type { ChronosForecastRequest, ChronosForecastResponse, ChronosHealthResponse } from "@/services/chronos.service";

const GATEWAY_URL = process.env.GATEWAY_INTERNAL_URL || process.env.NEXT_PUBLIC_API_URL || "http://localhost:8000";
const CHRONOS_URL = process.env.CHRONOS_API_URL || "http://localhost:9021";
const FORECAST_TIMEOUT_MS = 180_000;
const HEALTH_TIMEOUT_MS = 10_000;

export type ActionResult<T> =
  | { success: true; data: T }
  | { success: false; error: string };

type ForecastEndpoint = "daily" | "weekly";

interface ForecastDefaults {
  prediction_horizon: string;
  num_samples: number;
  confidence_levels: number[];
  aggregation_method: "mean" | "median" | "max" | "min";
}

const FORECAST_DEFAULTS: Record<ForecastEndpoint, ForecastDefaults> = {
  daily: {
    prediction_horizon: "4 hours",
    num_samples: 200,
    confidence_levels: [0.1, 0.2, 0.5, 0.8, 0.9],
    aggregation_method: "mean",
  },
  weekly: {
    prediction_horizon: "24 hours",
    num_samples: 200,
    confidence_levels: [0.1, 0.2, 0.5, 0.8, 0.9],
    aggregation_method: "mean",
  },
};

function extractErrorMessage(errorData: unknown): string {
  if (!errorData || typeof errorData !== "object") {
    return "Error desconocido del servidor";
  }

  const data = errorData as Record<string, unknown>;

  // FastAPI validation errors (array format)
  if (Array.isArray(data.detail)) {
    return data.detail
      .map((e: { loc?: (string | number)[]; msg?: string }) => {
        const field = e.loc?.slice(1).join(".") || "campo";
        return `${field}: ${e.msg || "error de validación"}`;
      })
      .join("; ");
  }

  if (typeof data.detail === "string") return data.detail;
  if (data.detail && typeof data.detail === "object") return JSON.stringify(data.detail);
  if (typeof data.message === "string") return data.message;

  return "Error desconocido del servidor";
}

async function fetchForecast(
  endpoint: ForecastEndpoint,
  request: ChronosForecastRequest
): Promise<ActionResult<ChronosForecastResponse>> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), FORECAST_TIMEOUT_MS);
  const defaults = FORECAST_DEFAULTS[endpoint];

  try {
    const response = await fetch(`${GATEWAY_URL}/api/v1/forecast/${endpoint}`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        ...request,
        prediction_horizon: request.prediction_horizon || defaults.prediction_horizon,
        num_samples: request.num_samples || defaults.num_samples,
        confidence_levels: request.confidence_levels || defaults.confidence_levels,
        aggregation_method: request.aggregation_method || defaults.aggregation_method,
        include_analysis: request.include_analysis !== false,
      }),
      signal: controller.signal,
      cache: "no-store",
    });

    clearTimeout(timeoutId);

    if (!response.ok) {
      const errorData = await response.json().catch(() => null);
      return {
        success: false,
        error: `Error de Chronos (${response.status}): ${extractErrorMessage(errorData)}`,
      };
    }

    return { success: true, data: await response.json() };
  } catch (err) {
    clearTimeout(timeoutId);

    if (err instanceof Error && err.name === "AbortError") {
      return {
        success: false,
        error: "La predicción tardó demasiado (>3 min). Intenta con un horizonte más corto.",
      };
    }

    const message = err instanceof Error ? err.message : "Error inesperado";
    return { success: false, error: `Error de conexión: ${message}` };
  }
}

export async function getChronosHealth(): Promise<ChronosHealthResponse> {
  const unavailable: ChronosHealthResponse = {
    status: "unavailable",
    gpu_available: false,
    model_loaded: false,
  };

  try {
    const controller = new AbortController();
    const timeoutId = setTimeout(() => controller.abort(), HEALTH_TIMEOUT_MS);

    const response = await fetch(`${CHRONOS_URL}/health`, {
      signal: controller.signal,
      cache: "no-store",
    });

    clearTimeout(timeoutId);

    return response.ok ? response.json() : unavailable;
  } catch {
    return unavailable;
  }
}

export async function getDailyForecast(request: ChronosForecastRequest) {
  return fetchForecast("daily", request);
}

export async function getWeeklyForecast(request: ChronosForecastRequest) {
  return fetchForecast("weekly", request);
}
