"use server";

import type { ChronosForecastRequest, ChronosForecastResponse, ChronosHealthResponse } from "@/services/chronos.service";

const CHRONOS_URL = process.env.CHRONOS_API_URL || "http://localhost:9021";

export async function getChronosHealth(): Promise<ChronosHealthResponse> {
  try {
    const response = await fetch(`${CHRONOS_URL}/health`);
    if (!response.ok) {
      return { status: "unavailable", gpu_available: false, model_loaded: false };
    }
    return response.json();
  } catch {
    return { status: "unavailable", gpu_available: false, model_loaded: false };
  }
}

export async function getDailyForecast(request: ChronosForecastRequest): Promise<ChronosForecastResponse> {
  const response = await fetch(`${CHRONOS_URL}/metrics/forecast/daily`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      ...request,
      prediction_horizon: request.prediction_horizon || "4 hours",
      num_samples: request.num_samples || 400,
      confidence_levels: request.confidence_levels || [0.1, 0.2, 0.5, 0.8, 0.9],
      aggregation_method: request.aggregation_method || "mean",
      include_analysis: request.include_analysis !== false,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ detail: "Unknown error" }));
    throw new Error(`Chronos error: ${error.detail}`);
  }

  return response.json();
}

export async function getWeeklyForecast(request: ChronosForecastRequest): Promise<ChronosForecastResponse> {
  const response = await fetch(`${CHRONOS_URL}/metrics/forecast/weekly`, {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({
      ...request,
      prediction_horizon: request.prediction_horizon || "24 hours",
      num_samples: request.num_samples || 400,
      confidence_levels: request.confidence_levels || [0.1, 0.2, 0.5, 0.8, 0.9],
      aggregation_method: request.aggregation_method || "mean",
      include_analysis: request.include_analysis !== false,
    }),
  });

  if (!response.ok) {
    const error = await response.json().catch(() => ({ detail: "Unknown error" }));
    throw new Error(`Chronos error: ${error.detail}`);
  }

  return response.json();
}

