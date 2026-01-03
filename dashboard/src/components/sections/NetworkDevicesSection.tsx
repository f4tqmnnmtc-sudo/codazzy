"use client";

import { useReducer, useRef, useTransition } from "react";
import { Trash2 } from "lucide-react";
import { getApiBaseUrl } from "@/lib/api-config";
import { cn } from "@/lib/utils";
import {
  Button,
  Input,
  Select,
  Label,
  Badge,
  ErrorBanner,
  EmptyState,
} from "@/components/ui/primitives";
import { ConfirmDialog } from "@/components/ui/Modal";

interface TelecoDevice {
  id: string;
  device_id: string;
  device_name: string;
  device_type: string;
  connection_config: {
    protocol: string;
    host: string;
    port: number;
  };
  status: string;
  last_seen?: string;
  metrics_count?: number;
  tags?: string[];
}

type FormState = {
  name: string;
  type: string;
  ip: string;
  protocol: string;
  snmpCommunity: string;
  snmpVersion: "v2c" | "v3";
  snmpPort: number;
  pollInterval: number;
};

type FormAction =
  | { type: "SET"; field: keyof FormState; value: string | number }
  | { type: "RESET" };

const INITIAL_FORM: FormState = {
  name: "",
  type: "router",
  ip: "",
  protocol: "snmp",
  snmpCommunity: "public",
  snmpVersion: "v2c",
  snmpPort: 161,
  pollInterval: 60,
};

const DEVICE_TYPES = [
  { id: "router", label: "Router", badge: "R" },
  { id: "switch", label: "Switch", badge: "S" },
  { id: "firewall", label: "Firewall", badge: "F" },
  { id: "ap", label: "AP", badge: "A" },
  { id: "other", label: "Otro", badge: "O" },
] as const;

const STATUS_COLORS: Record<string, string> = {
  online: "bg-emerald-500",
  warning: "bg-amber-500",
  maintenance: "bg-amber-500",
  offline: "bg-red-500",
  error: "bg-red-500",
};

function formReducer(state: FormState, action: FormAction): FormState {
  if (action.type === "RESET") return INITIAL_FORM;
  return { ...state, [action.field]: action.value };
}

function resolveDeviceBadge(tags?: string[]): string {
  if (!tags?.length) return "O";
  const type = tags[0]?.toLowerCase();
  return DEVICE_TYPES.find((d) => d.id === type)?.badge ?? "O";
}

export function NetworkDevicesSection() {
  const [devices, setDevices] = useReducer(
    (_: TelecoDevice[], next: TelecoDevice[]) => next,
    []
  );
  const [form, dispatch] = useReducer(formReducer, INITIAL_FORM);
  const [showForm, toggleForm] = useReducer((s: boolean) => !s, false);
  const [deleteTarget, setDeleteTarget] = useReducer(
    (_: string | null, next: string | null) => next,
    null
  );
  const [error, setError] = useReducer(
    (_: string | null, next: string | null) => next,
    null
  );

  const [isPending, startTransition] = useTransition();
  const hasLoaded = useRef(false);

  if (!hasLoaded.current) {
    hasLoaded.current = true;
    fetch(`${getApiBaseUrl()}/api/v1/teleco/devices`)
      .then((res) => (res.ok ? res.json() : { devices: [] }))
      .then((data) => setDevices(data.devices || []))
      .catch(() => setDevices([]));
  }

  async function submitDevice() {
    if (!form.name || !form.ip) {
      setError("Nombre e IP son requeridos");
      return;
    }

    setError(null);

    const payload = {
      device_id: form.name.toLowerCase().replace(/\s+/g, "-"),
      device_type: "standard",
      device_name: form.name,
      connection_config: {
        protocol: form.protocol === "http" ? "http_api" : form.protocol,
        host: form.ip,
        port: form.protocol === "snmp" ? form.snmpPort : 80,
        credentials:
          form.protocol === "snmp"
            ? { community: form.snmpCommunity, version: form.snmpVersion }
            : {},
        additional_params: {},
        timeout_seconds: 30,
        retry_attempts: 3,
      },
      metrics_config: {
        enabled_metrics: ["cpu", "memory", "interfaces", "uptime"],
        collection_interval: form.pollInterval,
        custom_oids: {},
        api_endpoints: [],
        ssh_commands: [],
      },
      tags: [form.type],
    };

    startTransition(async () => {
      try {
        const res = await fetch(`${getApiBaseUrl()}/api/v1/teleco/devices`, {
          method: "POST",
          headers: { "Content-Type": "application/json" },
          body: JSON.stringify(payload),
        });

        if (res.ok) {
          const refreshRes = await fetch(`${getApiBaseUrl()}/api/v1/teleco/devices`);
          if (refreshRes.ok) {
            const data = await refreshRes.json();
            setDevices(data.devices || []);
          }
          dispatch({ type: "RESET" });
          toggleForm();
        } else {
          const err = await res.json().catch(() => ({}));
          setError(err.message || err.detail || "Error agregando dispositivo");
        }
      } catch {
        setError("Error de conexión con el servidor");
      }
    });
  }

  async function removeDevice() {
    if (!deleteTarget) return;

    try {
      const res = await fetch(
        `${getApiBaseUrl()}/api/v1/teleco/devices/${deleteTarget}`,
        { method: "DELETE" }
      );
      if (res.ok) {
        setDevices(devices.filter((d) => d.id !== deleteTarget));
      }
    } catch {
      setError("Error eliminando dispositivo");
    } finally {
      setDeleteTarget(null);
    }
  }

  const targetDevice = devices.find((d) => d.id === deleteTarget);

  return (
    <div className="space-y-4">
      <header className="flex items-center justify-between">
        <h4 className="text-[14px] font-medium text-white">
          Dispositivos de Red ({devices.length})
        </h4>
        <Button variant={showForm ? "secondary" : "primary"} size="sm" onClick={toggleForm}>
          {showForm ? "Cancelar" : "+ Agregar"}
        </Button>
      </header>

      {showForm && (
        <div className="p-4 rounded-lg bg-[#131a26] border border-[#2a3548] space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label>Nombre</Label>
              <Input
                value={form.name}
                onChange={(e) => dispatch({ type: "SET", field: "name", value: e.target.value })}
                placeholder="Router Principal"
              />
            </div>
            <div>
              <Label>IP</Label>
              <Input
                value={form.ip}
                onChange={(e) => dispatch({ type: "SET", field: "ip", value: e.target.value })}
                placeholder="192.168.1.1"
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <Label>Tipo</Label>
              <Select
                value={form.type}
                onChange={(e) => dispatch({ type: "SET", field: "type", value: e.target.value })}
              >
                {DEVICE_TYPES.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.label}
                  </option>
                ))}
              </Select>
            </div>
            <div>
              <Label>Protocolo</Label>
              <Select
                value={form.protocol}
                onChange={(e) => dispatch({ type: "SET", field: "protocol", value: e.target.value })}
              >
                <option value="snmp">SNMP</option>
                <option value="http">HTTP/REST</option>
                <option value="mqtt">MQTT</option>
              </Select>
            </div>
          </div>

          {form.protocol === "snmp" && (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <Label>Community</Label>
                <Input
                  value={form.snmpCommunity}
                  onChange={(e) =>
                    dispatch({ type: "SET", field: "snmpCommunity", value: e.target.value })
                  }
                  placeholder="public"
                />
              </div>
              <div>
                <Label>Versión</Label>
                <Select
                  value={form.snmpVersion}
                  onChange={(e) =>
                    dispatch({ type: "SET", field: "snmpVersion", value: e.target.value })
                  }
                >
                  <option value="v2c">v2c</option>
                  <option value="v3">v3</option>
                </Select>
              </div>
            </div>
          )}

          <div>
            <Label>Intervalo (segundos)</Label>
            <Input
              type="number"
              value={form.pollInterval}
              onChange={(e) =>
                dispatch({ type: "SET", field: "pollInterval", value: parseInt(e.target.value) || 60 })
              }
              min={10}
              max={3600}
            />
          </div>

          {error && <ErrorBanner message={error} onDismiss={() => setError(null)} />}

          <Button
            variant="primary"
            onClick={submitDevice}
            loading={isPending}
            className="w-full"
          >
            Agregar Dispositivo
          </Button>
        </div>
      )}

      {devices.length === 0 ? (
        <EmptyState
          title="Sin dispositivos"
          description="No hay dispositivos de red configurados"
        />
      ) : (
        <ul className="space-y-2">
          {devices.map((device) => (
            <li
              key={device.id}
              className="p-3 rounded-lg bg-[#0a0e17] border border-[#2a3548] hover:border-[#3a4558] transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <span className="w-8 h-8 rounded-lg bg-[#2a3548] flex items-center justify-center text-[12px] font-bold text-emerald-400">
                    {resolveDeviceBadge(device.tags)}
                  </span>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium text-white">
                        {device.device_name}
                      </span>
                      <span
                        className={cn(
                          "w-2 h-2 rounded-full",
                          STATUS_COLORS[device.status?.toLowerCase()] ?? "bg-gray-500"
                        )}
                      />
                    </div>
                    <div className="text-[11px] text-[#8b95a5]">
                      {device.connection_config?.host}:{device.connection_config?.port} |{" "}
                      {device.connection_config?.protocol?.toUpperCase()}
                    </div>
                  </div>
                </div>
                <button
                  type="button"
                  onClick={() => setDeleteTarget(device.id)}
                  className="p-1.5 text-[#8b95a5] hover:text-red-400 hover:bg-red-500/10 rounded transition-colors"
                >
                  <Trash2 className="w-4 h-4" />
                </button>
              </div>

              {device.metrics_count !== undefined && device.metrics_count > 0 && (
                <div className="mt-3 text-[11px] text-[#8b95a5]">
                  Métricas recolectadas: {device.metrics_count}
                </div>
              )}
            </li>
          ))}
        </ul>
      )}

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={removeDevice}
        title="Eliminar dispositivo"
        description={`¿Eliminar ${targetDevice?.device_name || "este dispositivo"}?`}
        variant="danger"
        confirmText="Eliminar"
      />
    </div>
  );
}
