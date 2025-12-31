import { getApiBaseUrl } from "@/lib/api-config";

const TIMEOUT = 10000;

async function request<T>(url: string, options: RequestInit = {}): Promise<T> {
  const controller = new AbortController();
  const timeoutId = setTimeout(() => controller.abort(), TIMEOUT);

  try {
    const response = await fetch(url, {
      ...options,
      signal: controller.signal,
      headers: { "Content-Type": "application/json", ...options.headers },
    });
    clearTimeout(timeoutId);

    if (!response.ok) {
      const error = await response.json().catch(() => ({}));
      throw new Error(error.detail || error.error?.message || `HTTP ${response.status}`);
    }

    return response.json();
  } catch (error) {
    clearTimeout(timeoutId);
    if (error instanceof Error && error.name === "AbortError") {
      throw new Error("Request timeout");
    }
    throw error;
  }
}

class GatewayService {
  private get baseUrl(): string {
    return getApiBaseUrl();
  }

  private url(path: string): string {
    return `${this.baseUrl}${path}`;
  }

  async get<T>(endpoint: string): Promise<T> {
    const url = endpoint.startsWith("http") ? endpoint : this.url(endpoint);
    return request<T>(url);
  }

  async post<T>(endpoint: string, data?: unknown): Promise<T> {
    const url = endpoint.startsWith("http") ? endpoint : this.url(endpoint);
    return request<T>(url, { method: "POST", body: data ? JSON.stringify(data) : undefined });
  }

  async getInstalledServers(): Promise<unknown[]> {
    try {
      const data = await this.get<{ servers?: unknown[] }>("/api/v1/agents/installed-servers");
      return data.servers || [];
    } catch {
      return [];
    }
  }

  async fetchRemoteConfig(connectionData: {
    hostname: string;
    port?: number;
    username: string;
    password?: string;
    private_key?: string;
    config_path?: string;
  }): Promise<unknown> {
    return this.post("/api/v1/agents/remote-config/fetch", {
      hostname: connectionData.hostname,
      port: connectionData.port || 22,
      username: connectionData.username,
      password: connectionData.password,
      private_key: connectionData.private_key,
      config_path: connectionData.config_path,
    });
  }

  async saveRemoteConfig(updateData: {
    hostname: string;
    port?: number;
    username: string;
    password?: string;
    private_key?: string;
    config_content: string;
    restart_agent?: boolean;
    config_path?: string;
    agent_path?: string;
  }): Promise<unknown> {
    return this.post("/api/v1/agents/remote-config/update", {
      hostname: updateData.hostname,
      port: updateData.port || 22,
      username: updateData.username,
      password: updateData.password,
      private_key: updateData.private_key,
      config_content: updateData.config_content,
      restart_agent: updateData.restart_agent ?? true,
      config_path: updateData.config_path,
      agent_path: updateData.agent_path,
    });
  }

  async createRemoteInstallJob(installRequest: unknown): Promise<{ job_id?: string }> {
    return this.post("/api/v1/agents/remote-install", installRequest);
  }

  async getInstallJobStatus(jobId: string): Promise<unknown> {
    return this.get(`/api/v1/agents/remote-install/${jobId}`);
  }

  async getCachedPredictions(): Promise<{
    predictions: Array<{
      id: string;
      device_id: string;
      device_name: string;
      metric_name: string;
      display_name: string | null;
      current_value: number;
      predicted_value: number;
      threshold_value: number;
      threshold_type: string;
      predicted_at: string;
      confidence: number;
      trend: string;
      hours_until: number;
    }>;
    count: number;
  }> {
    try {
      return await this.get("/api/v1/alerts/predictions");
    } catch {
      return { predictions: [], count: 0 };
    }
  }

  async testSSHConnection(connectionData: {
    hostname: string;
    port?: number;
    username: string;
    password?: string;
    private_key?: string;
  }): Promise<{ status: string; error?: string; os?: string }> {
    try {
      const params = new URLSearchParams({ username: connectionData.username });
      if (connectionData.port) params.append("port", connectionData.port.toString());
      if (connectionData.password) params.append("password", connectionData.password);
      if (connectionData.private_key) params.append("private_key", connectionData.private_key);

      return this.get(`/api/v1/agents/health-check/${connectionData.hostname}?${params}`);
    } catch (error) {
      return { status: "failed", error: error instanceof Error ? error.message : "Connection test failed" };
    }
  }
}

export const gatewayService = new GatewayService();
