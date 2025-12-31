"use client";
import { getApiBaseUrl } from "@/lib/api-config";

import { useState, useEffect } from "react";

interface NetworkDevice {
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

interface DeviceFormData {
  name: string;
  type: string;
  ip: string;
  protocol: string;
  snmpCommunity?: string;
  snmpVersion?: "v2c" | "v3";
  snmpPort?: number;
  httpEndpoint?: string;
  mqttTopic?: string;
  pollInterval: number;
}

const getApiBase = () => getApiBaseUrl();

export function NetworkDevicesSection() {
  const [devices, setDevices] = useState<NetworkDevice[]>([]);
  const [isLoading, setIsLoading] = useState(true);
  const [showAddForm, setShowAddForm] = useState(false);
  const [formData, setFormData] = useState<DeviceFormData>({
    name: "",
    type: "router",
    ip: "",
    protocol: "snmp",
    snmpCommunity: "public",
    snmpVersion: "v2c",
    snmpPort: 161,
    pollInterval: 60,
  });
  const [error, setError] = useState<string | null>(null);
  const [isSubmitting, setIsSubmitting] = useState(false);

  const deviceTypes = [
    { id: "router", name: "Router", icon: "R" },
    { id: "switch", name: "Switch", icon: "S" },
    { id: "firewall", name: "Firewall", icon: "F" },
    { id: "ap", name: "AP", icon: "A" },
    { id: "other", name: "Otro", icon: "O" },
  ];

  useEffect(() => {
    loadDevices();
  }, []);

  const loadDevices = async () => {
    setIsLoading(true);
    try {
      const res = await fetch(`${getApiBase()}/api/v1/teleco/devices`);
      if (res.ok) {
        const data = await res.json();
        setDevices(data.devices || []);
      } else {
        setDevices([]);
      }
    } catch (err) {
      console.error("Error loading network devices:", err);
      setDevices([]);
    } finally {
      setIsLoading(false);
    }
  };

  const addDevice = async () => {
    if (!formData.name || !formData.ip) {
      setError("Nombre e IP son requeridos");
      return;
    }

    setIsSubmitting(true);
    setError(null);

    // Mapear tipo del frontend al formato del gateway
    const deviceTypeMap: Record<string, string> = {
      router: "standard",
      switch: "standard",
      firewall: "standard",
      ap: "standard",
      other: "standard",
    };

    // Mapear protocolo del frontend al formato del gateway
    const protocolMap: Record<string, string> = {
      snmp: "snmp",
      http: "http_api",
      mqtt: "mqtt",
    };

    // Construir credentials para SNMP
    const credentials: Record<string, string> = {};
    if (formData.protocol === "snmp") {
      credentials.community = formData.snmpCommunity || "public";
      credentials.version = formData.snmpVersion || "v2c";
    }

    // Construir el request en el formato que espera el gateway
    const requestBody = {
      device_id: formData.name.toLowerCase().replace(/\s+/g, "-"),
      device_type: deviceTypeMap[formData.type] || "standard",
      device_name: formData.name,
      connection_config: {
        protocol: protocolMap[formData.protocol] || "snmp",
        host: formData.ip,
        port: formData.protocol === "snmp" ? (formData.snmpPort || 161) : 80,
        credentials: credentials,
        additional_params: {},
        timeout_seconds: 30,
        retry_attempts: 3,
      },
      metrics_config: {
        enabled_metrics: ["cpu", "memory", "interfaces", "uptime"],
        collection_interval: formData.pollInterval,
        custom_oids: {},
        api_endpoints: [],
        ssh_commands: [],
      },
      tags: [formData.type],
    };

    try {
      const res = await fetch(`${getApiBase()}/api/v1/teleco/devices`, {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify(requestBody),
      });

      if (res.ok) {
        await loadDevices();
        setShowAddForm(false);
        resetForm();
      } else {
        const errData = await res.json();
        setError(errData.message || errData.detail || "Error agregando dispositivo");
      }
    } catch (err) {
      setError("Error de conexion con el servidor");
    } finally {
      setIsSubmitting(false);
    }
  };

  const deleteDevice = async (id: string) => {
    if (!confirm("Eliminar este dispositivo?")) return;

    try {
      const res = await fetch(`${getApiBase()}/api/v1/teleco/devices/${id}`, { method: "DELETE" });
      if (res.ok) {
        setDevices((prev) => prev.filter((d) => d.id !== id));
      }
    } catch (err) {
      console.error("Error deleting device:", err);
    }
  };

  const resetForm = () => {
    setFormData({
      name: "",
      type: "router",
      ip: "",
      protocol: "snmp",
      snmpCommunity: "public",
      snmpVersion: "v2c",
      snmpPort: 161,
      pollInterval: 60,
    });
  };

  const getStatusColor = (status: string) => {
    const statusLower = status?.toLowerCase() || "";
    switch (statusLower) {
      case "online":
        return "bg-emerald-500";
      case "warning":
      case "maintenance":
        return "bg-amber-500";
      case "offline":
      case "error":
        return "bg-red-500";
      default:
        return "bg-gray-500";
    }
  };

  const getDeviceTypeIcon = (tags?: string[]) => {
    if (!tags || tags.length === 0) return "O";
    const type = tags[0]?.toLowerCase();
    const typeMap: Record<string, string> = {
      router: "R",
      switch: "S",
      firewall: "F",
      ap: "A",
    };
    return typeMap[type] || "O";
  };

  const inputClass =
    "w-full px-3 py-2 rounded-lg border border-[#2a3548] bg-[#0a0e17] text-[13px] text-white focus:outline-none focus:ring-2 focus:ring-emerald-500 placeholder-[#8b95a5]";
  const labelClass = "block text-[12px] text-[#8b95a5] uppercase tracking-wide mb-1.5";

  return (
    <div className="space-y-4">
      {/* Header */}
      <div className="flex items-center justify-between">
        <h4 className="text-[14px] font-medium text-white">
          Dispositivos de Red ({devices.length})
        </h4>
        <button
          onClick={() => setShowAddForm(!showAddForm)}
          className="px-3 py-1.5 bg-emerald-500 hover:bg-emerald-600 text-[#0a0e17] text-[12px] font-medium rounded-lg transition-colors"
        >
          {showAddForm ? "Cancelar" : "+ Agregar"}
        </button>
      </div>

      {/* Add Form */}
      {showAddForm && (
        <div className="p-4 rounded-lg bg-[#131a26] border border-[#2a3548] space-y-3">
          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className={labelClass}>Nombre</label>
              <input
                type="text"
                value={formData.name}
                onChange={(e) => setFormData({ ...formData, name: e.target.value })}
                placeholder="Router Principal"
                className={inputClass}
              />
            </div>
            <div>
              <label className={labelClass}>IP</label>
              <input
                type="text"
                value={formData.ip}
                onChange={(e) => setFormData({ ...formData, ip: e.target.value })}
                placeholder="192.168.1.1"
                className={inputClass}
              />
            </div>
          </div>

          <div className="grid grid-cols-2 gap-3">
            <div>
              <label className={labelClass}>Tipo</label>
              <select
                value={formData.type}
                onChange={(e) => setFormData({ ...formData, type: e.target.value })}
                className={inputClass}
              >
                {deviceTypes.map((t) => (
                  <option key={t.id} value={t.id}>
                    {t.name}
                  </option>
                ))}
              </select>
            </div>
            <div>
              <label className={labelClass}>Protocolo</label>
              <select
                value={formData.protocol}
                onChange={(e) => setFormData({ ...formData, protocol: e.target.value })}
                className={inputClass}
              >
                <option value="snmp">SNMP</option>
                <option value="http">HTTP/REST</option>
                <option value="mqtt">MQTT</option>
              </select>
            </div>
          </div>

          {formData.protocol === "snmp" && (
            <div className="grid grid-cols-2 gap-3">
              <div>
                <label className={labelClass}>Community</label>
                <input
                  type="text"
                  value={formData.snmpCommunity}
                  onChange={(e) => setFormData({ ...formData, snmpCommunity: e.target.value })}
                  placeholder="public"
                  className={inputClass}
                />
              </div>
              <div>
                <label className={labelClass}>Version</label>
                <select
                  value={formData.snmpVersion}
                  onChange={(e) => setFormData({ ...formData, snmpVersion: e.target.value as "v2c" | "v3" })}
                  className={inputClass}
                >
                  <option value="v2c">v2c</option>
                  <option value="v3">v3</option>
                </select>
              </div>
            </div>
          )}

          <div>
            <label className={labelClass}>Intervalo (segundos)</label>
            <input
              type="number"
              value={formData.pollInterval}
              onChange={(e) => setFormData({ ...formData, pollInterval: parseInt(e.target.value) })}
              min={10}
              max={3600}
              className={inputClass}
            />
          </div>

          {error && (
            <div className="p-2 rounded bg-red-500/10 border border-red-500/30">
              <p className="text-[12px] text-red-400">{error}</p>
            </div>
          )}

          <button
            onClick={addDevice}
            disabled={isSubmitting}
            className="w-full py-2 bg-emerald-500 hover:bg-emerald-600 text-[#0a0e17] text-[13px] font-medium rounded-lg disabled:opacity-50 transition-colors"
          >
            {isSubmitting ? "Agregando..." : "Agregar Dispositivo"}
          </button>
        </div>
      )}

      {/* Devices List */}
      {isLoading ? (
        <div className="text-center py-8 text-[#8b95a5]">Cargando dispositivos...</div>
      ) : devices.length === 0 ? (
        <div className="text-center py-8 text-[#8b95a5]">
          No hay dispositivos de red configurados
        </div>
      ) : (
        <div className="space-y-2">
          {devices.map((device) => (
            <div
              key={device.id}
              className="p-3 rounded-lg bg-[#0a0e17] border border-[#2a3548] hover:border-[#3a4558] transition-colors"
            >
              <div className="flex items-start justify-between">
                <div className="flex items-center gap-3">
                  <span className="w-8 h-8 rounded-lg bg-[#2a3548] flex items-center justify-center text-[12px] font-bold text-emerald-400">
                    {getDeviceTypeIcon(device.tags)}
                  </span>
                  <div>
                    <div className="flex items-center gap-2">
                      <span className="text-[13px] font-medium text-white">{device.device_name}</span>
                      <span className={`w-2 h-2 rounded-full ${getStatusColor(device.status)}`} />
                    </div>
                    <div className="text-[11px] text-[#8b95a5]">
                      {device.connection_config?.host}:{device.connection_config?.port} | {device.connection_config?.protocol?.toUpperCase()}
                    </div>
                  </div>
                </div>
                <button
                  onClick={() => deleteDevice(device.id)}
                  className="p-1.5 text-[#8b95a5] hover:text-red-400 hover:bg-red-500/10 rounded transition-colors"
                >
                  <svg width="14" height="14" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
                    <path d="M3 6h18M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
                  </svg>
                </button>
              </div>

              {device.metrics_count !== undefined && device.metrics_count > 0 && (
                <div className="mt-3 flex gap-4 text-[11px] text-[#8b95a5]">
                  <span>Cantidad de datos recolectados: {device.metrics_count}</span>
                </div>
              )}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
