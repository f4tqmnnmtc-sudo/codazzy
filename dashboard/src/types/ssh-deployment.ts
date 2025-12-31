export type JobStatus = "pending" | "running" | "completed" | "failed" | "cancelled";
export type OsType = "linux" | "windows" | "auto" | "unknown";
export type Environment = "development" | "staging" | "production";
export type AuthMethod = "password" | "key";

export interface RemoteInstallRequest {
  hostname: string;
  port: number;
  username: string;
  password?: string;
  private_key?: string;
  os_type: string;
  nats_url: string;
  node_id?: string;
  location?: string;
  environment: string;
  tags: string[];
}

export interface InstallJobResponse {
  job_id: string;
  hostname: string;
  status: JobStatus;
  progress: number;
  current_step: string;
  created_at: string;
  started_at?: string;
  completed_at?: string;
  error_message?: string;
  logs: string[];
}

export interface CreateJobResponse {
  job_id: string;
  status: "pending";
  message: string;
}

export interface SSHHealthCheckResponse {
  status: "ok" | "failed";
  hostname?: string;
  os?: OsType;
  error?: string;
}

export interface InstallFormState {
  hostname: string;
  port: string;
  username: string;
  password: string;
  privateKey: string;
  authMethod: AuthMethod;
  osType: OsType;
  natsUrl: string;
  nodeId: string;
  location: string;
  environment: Environment;
  tags: string;
}

export const INITIAL_FORM_STATE: InstallFormState = {
  hostname: "",
  port: "22",
  username: "",
  password: "",
  privateKey: "",
  authMethod: "password",
  osType: "auto",
  natsUrl: "nats://nats:4222",
  nodeId: "",
  location: "",
  environment: "production",
  tags: "",
};

export const JOB_STATUS_LABELS: Record<JobStatus, string> = {
  pending: "Iniciado",
  running: "Corriendo",
  completed: "Completado",
  failed: "Fallido",
  cancelled: "Cancelado",
};

const parsePort = (raw: string) => parseInt(raw, 10) || 22;

const parseTags = (raw: string): string[] =>
  raw ? raw.split(",").map((t) => t.trim()).filter(Boolean) : [];

export function toInstallRequest({
  hostname,
  port,
  username,
  password,
  privateKey,
  authMethod,
  osType,
  natsUrl,
  nodeId,
  location,
  environment,
  tags,
}: InstallFormState): RemoteInstallRequest {
  const usePassword = authMethod === "password";

  return {
    hostname,
    port: parsePort(port),
    username,
    ...(usePassword ? { password } : { private_key: privateKey }),
    os_type: osType,
    nats_url: natsUrl,
    node_id: nodeId || `${hostname}-agent`,
    location: location || "remote",
    environment,
    tags: parseTags(tags),
  };
}

export function toSSHTestParams({
  hostname,
  port,
  username,
  password,
  privateKey,
  authMethod,
}: Pick<InstallFormState, "hostname" | "port" | "username" | "password" | "privateKey" | "authMethod">) {
  return {
    hostname,
    port: parsePort(port),
    username,
    ...(authMethod === "password" ? { password } : { private_key: privateKey }),
  };
}
