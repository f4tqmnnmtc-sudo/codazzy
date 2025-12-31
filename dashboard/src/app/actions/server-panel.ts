"use server";

import { getApiBaseUrl } from "@/lib/api-config";
import type { ServerDocument } from "@/types/gateway";

export async function getServerDocuments(agentId: string): Promise<ServerDocument[]> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/servers/${agentId}/documents`, { cache: "no-store" });
    if (!res.ok) return [];
    const data = await res.json();
    return data.documents || [];
  } catch {
    return [];
  }
}

export async function uploadServerDocument(
  agentId: string,
  doc: {
    filename: string;
    file_type: string;
    file_size: number;
    content: string;
  }
): Promise<ServerDocument | null> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/servers/${agentId}/documents/upload`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ ...doc, description: null }),
    });
    if (!res.ok) return null;
    const data = await res.json();
    return data.document || null;
  } catch {
    return null;
  }
}

export async function deleteServerDocument(docId: number): Promise<boolean> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/documents/${docId}`, { method: "DELETE" });
    return res.ok;
  } catch {
    return false;
  }
}

export async function savePredictions(
  nodeId: string,
  metricType: string,
  predictions: Array<{
    timestamp: number;
    value: number;
    lower_bound: number;
    upper_bound: number;
    confidence: number;
  }>
): Promise<boolean> {
  try {
    const API = getApiBaseUrl();
    const res = await fetch(`${API}/api/v1/predictions`, {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({
        node_id: nodeId,
        metric_type: metricType,
        model_type: "chronos",
        predictions,
      }),
    });
    return res.ok;
  } catch {
    return false;
  }
}
