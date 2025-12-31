"use client";

import { useRef, useState, useCallback, type FormEvent } from "react";
import { gatewayService } from "@/services/gateway.service";
import {
  Button,
  Input,
  Select,
  Label,
  Badge,
  Card,
  ProgressBar,
  ErrorBanner,
} from "@/components/ui/primitives";
import {
  type InstallFormState,
  type InstallJobResponse,
  type JobStatus,
  INITIAL_FORM_STATE,
  JOB_STATUS_LABELS,
  toInstallRequest,
  toSSHTestParams,
} from "@/types/ssh-deployment";

type ConnectionStatus = "idle" | "success" | "failed";

interface AgentInstallSectionProps {
  onInstallComplete?: () => void;
}

const STATUS_TO_CARD_VARIANT: Record<JobStatus, "success" | "error" | "default"> = {
  completed: "success",
  failed: "error",
  pending: "default",
  running: "default",
  cancelled: "default",
};

const STATUS_TO_BADGE_VARIANT: Record<JobStatus, "success" | "error" | "warning" | "default"> = {
  completed: "success",
  failed: "error",
  pending: "warning",
  running: "warning",
  cancelled: "default",
};

function JobCard({ job }: { job: InstallJobResponse }) {
  const cardVariant = STATUS_TO_CARD_VARIANT[job.status];
  const badgeVariant = STATUS_TO_BADGE_VARIANT[job.status];

  return (
    <Card variant={cardVariant} padding="sm">
      <header className="flex items-center justify-between mb-2">
        <span className="text-[13px] font-medium text-white">{job.hostname}</span>
        <Badge variant={badgeVariant}>{JOB_STATUS_LABELS[job.status]}</Badge>
      </header>

      {job.status === "running" && (
        <>
          <ProgressBar value={job.progress} variant="success" className="mb-1" />
          <p className="text-[11px] text-[#8b95a5]">{job.current_step}</p>
        </>
      )}

      {job.error_message && (
        <p className="text-[11px] text-red-400 mt-1">{job.error_message}</p>
      )}
    </Card>
  );
}

interface FormFieldProps {
  label: string;
  children: React.ReactNode;
  className?: string;
}

function FormField({ label, children, className }: FormFieldProps) {
  return (
    <div className={className}>
      <Label>{label}</Label>
      {children}
    </div>
  );
}

export function AgentInstallSection({ onInstallComplete }: AgentInstallSectionProps) {
  const [form, setForm] = useState(INITIAL_FORM_STATE);
  const [jobs, setJobs] = useState<InstallJobResponse[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [testing, setTesting] = useState(false);
  const [connection, setConnection] = useState<ConnectionStatus>("idle");
  const [error, setError] = useState<string | null>(null);

  const pollTimers = useRef(new Map<string, NodeJS.Timeout>());

  const updateField = useCallback(<K extends keyof InstallFormState>(
    field: K,
    value: InstallFormState[K]
  ) => {
    setForm((prev) => {
      const next = { ...prev, [field]: value };
      if (field === "hostname" && value && !prev.nodeId) {
        next.nodeId = String(value).replace(/[^a-zA-Z0-9-]/g, "-");
      }
      return next;
    });
  }, []);

  const pollJob = useCallback((jobId: string) => {
    const tick = async () => {
      try {
        const status = await gatewayService.getInstallJobStatus(jobId) as InstallJobResponse;
        setJobs((prev) => prev.map((j) => (j.job_id === jobId ? status : j)));

        const terminal = ["completed", "failed", "cancelled"].includes(status.status);
        if (terminal) {
          pollTimers.current.delete(jobId);
          if (status.status === "completed") onInstallComplete?.();
          return;
        }

        pollTimers.current.set(jobId, setTimeout(tick, 1000));
      } catch {
        pollTimers.current.delete(jobId);
      }
    };
    tick();
  }, [onInstallComplete]);

  const testConnection = async () => {
    const { hostname, username, port, authMethod, password } = form;

    if (!hostname || !username || !port) {
      setError("Hostname, puerto y usuario son requeridos");
      return;
    }
    if (authMethod === "password" && !password) {
      setError("Password requerido");
      return;
    }

    setTesting(true);
    setError(null);
    setConnection("idle");

    try {
      const res = await gatewayService.testSSHConnection(toSSHTestParams(form));

      if (res.status === "ok") {
        setConnection("success");
        if (res.os && form.osType === "auto") {
          setForm((prev) => ({ ...prev, osType: res.os as "linux" | "windows" }));
        }
      } else {
        setConnection("failed");
        setError(res.error ?? "Conexión fallida");
      }
    } catch {
      setConnection("failed");
      setError("Error probando conexión SSH");
    } finally {
      setTesting(false);
    }
  };

  const handleSubmit = async (e: FormEvent) => {
    e.preventDefault();
    if (submitting) return;

    setSubmitting(true);
    setError(null);

    try {
      const { job_id } = await gatewayService.createRemoteInstallJob(toInstallRequest(form));

      if (job_id) {
        const newJob: InstallJobResponse = {
          job_id,
          hostname: form.hostname,
          status: "pending",
          progress: 0,
          current_step: "Iniciando instalación...",
          created_at: new Date().toISOString(),
          logs: [],
        };

        setJobs((prev) => [...prev, newJob]);
        pollJob(job_id);
        setForm(INITIAL_FORM_STATE);
        setConnection("idle");
      }
    } catch (err) {
      setError(err instanceof Error ? err.message : "Error iniciando instalación");
    } finally {
      setSubmitting(false);
    }
  };

  const canSubmit = connection === "success" && !submitting;
  const canTest = !testing && !!form.hostname && !!form.username;

  return (
    <div className="space-y-4">
      {jobs.length > 0 && (
        <div className="space-y-2">
          {jobs.map((job) => <JobCard key={job.job_id} job={job} />)}
        </div>
      )}

      <form onSubmit={handleSubmit} className="space-y-4">
        <div className="grid grid-cols-3 gap-3">
          <FormField label="Hostname / IP" className="col-span-2">
            <Input
              value={form.hostname}
              onChange={(e) => updateField("hostname", e.target.value)}
              placeholder="192.168.1.100"
              required
            />
          </FormField>
          <FormField label="Puerto">
            <Input
              type="number"
              value={form.port}
              onChange={(e) => updateField("port", e.target.value)}
              required
            />
          </FormField>
        </div>

        <FormField label="Usuario SSH">
          <Input
            value={form.username}
            onChange={(e) => updateField("username", e.target.value)}
            placeholder="usuario"
            required
          />
        </FormField>

        <FormField label="Autenticación">
          <Input
            type="password"
            value={form.password}
            onChange={(e) => updateField("password", e.target.value)}
            placeholder="contraseña"
          />
        </FormField>

        <div className="flex items-center gap-3">
          <Button
            type="button"
            variant="secondary"
            onClick={testConnection}
            disabled={!canTest}
            loading={testing}
          >
            Probar
          </Button>

          {connection === "success" && (
            <span className="text-[13px] text-emerald-400">✓ Conectado</span>
          )}
          {connection === "failed" && (
            <span className="text-[13px] text-red-400">✗ Error</span>
          )}
        </div>

        <div className="grid grid-cols-2 gap-3">
          <FormField label="Sistema Operativo">
            <Select
              value={form.osType}
              onChange={(e) => updateField("osType", e.target.value as InstallFormState["osType"])}
            >
              <option value="auto">Auto-detectar</option>
              <option value="linux">Linux</option>
              <option value="windows">Windows</option>
            </Select>
          </FormField>
          <FormField label="Entorno">
            <Select
              value={form.environment}
              onChange={(e) => updateField("environment", e.target.value as InstallFormState["environment"])}
            >
              <option value="development">Development</option>
              <option value="staging">Staging</option>
              <option value="production">Production</option>
            </Select>
          </FormField>
        </div>

        <FormField label="NATS URL">
          <Input
            value={form.natsUrl}
            onChange={(e) => updateField("natsUrl", e.target.value)}
          />
        </FormField>

        <div className="grid grid-cols-2 gap-3">
          <FormField label="Node ID">
            <Input
              value={form.nodeId}
              onChange={(e) => updateField("nodeId", e.target.value)}
              placeholder="auto-generado"
            />
          </FormField>
          <FormField label="Ubicación">
            <Input
              value={form.location}
              onChange={(e) => updateField("location", e.target.value)}
              placeholder="datacenter-1"
            />
          </FormField>
        </div>

        <FormField label="Etiquetas">
          <Input
            value={form.tags}
            onChange={(e) => updateField("tags", e.target.value)}
            placeholder="web, production, nginx"
          />
        </FormField>

        {error && <ErrorBanner message={error} onDismiss={() => setError(null)} />}

        <Button type="submit" disabled={!canSubmit} loading={submitting} className="w-full">
          Instalar Agente
        </Button>
      </form>
    </div>
  );
}
