"use client";

import { useState, useEffect } from "react";
import { gatewayService } from "@/services/gateway.service";

interface InstalledServer {
  node_id: string;
  hostname: string;
  status: string;
  os_type: string;
  config_path: string;
  agent_path: string;
  ssh_port: number;
  ssh_username?: string;
  location?: string;
  environment: string;
  has_connection: boolean;
}

interface ConnectionData {
  hostname: string;
  port: number;
  username: string;
  password: string;
  config_path: string;
}

interface ConfigEditorSectionProps {
  serverId?: string;
  onClose?: () => void;
}

export function ConfigEditorSection({ serverId, onClose }: ConfigEditorSectionProps) {
  const [servers, setServers] = useState<InstalledServer[]>([]);
  const [selectedServer, setSelectedServer] = useState<InstalledServer | null>(null);
  const [connectionData, setConnectionData] = useState<ConnectionData>({
    hostname: "",
    port: 22,
    username: "",
    password: "",
    config_path: "/etc/codazzy/agent/config.toml",
  });
  const [configContent, setConfigContent] = useState<string>("");
  const [isLoading, setIsLoading] = useState(false);
  const [isLoadingServers, setIsLoadingServers] = useState(true);
  const [isSaving, setIsSaving] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [success, setSuccess] = useState<string | null>(null);
  const [showConnectionForm, setShowConnectionForm] = useState(false);

  useEffect(() => {
    loadServers();
  }, []);

  // Seleccionar con server serverId siempre y cuando exista
  useEffect(() => {
    if (serverId && servers.length > 0) {
      const server = servers.find((s) => s.node_id === serverId);
      if (server) {
        handleSelectServer(server);
      }
    }
  }, [serverId, servers]);

  const loadServers = async () => {
    setIsLoadingServers(true);
    try {
      const data = await gatewayService.getInstalledServers();
      setServers(data || []);
    } catch (err) {
      console.error("Error loading servers:", err);
      setServers([]);
    } finally {
      setIsLoadingServers(false);
    }
  };

  const handleSelectServer = (server: InstalledServer) => {
    setSelectedServer(server);
    setError(null);
    setSuccess(null);
    setConfigContent("");

    // Load saved preferences or use server defaults
    const savedPrefs = localStorage.getItem(`codazzy-config-prefs-${server.node_id}`);
    if (savedPrefs) {
      const prefs = JSON.parse(savedPrefs);
      setConnectionData({
        hostname: prefs.hostname || server.hostname,
        port: prefs.port || server.ssh_port || 22,
        username: prefs.username || server.ssh_username || "",
        password: "",
        config_path: prefs.config_path || server.config_path || "/etc/codazzy/agent/config.toml",
      });
      setShowConnectionForm(false);
    } else {
      setConnectionData({
        hostname: server.hostname,
        port: server.ssh_port || 22,
        username: server.ssh_username || "",
        password: "",
        config_path: server.config_path || "/etc/codazzy/agent/config.toml",
      });
      setShowConnectionForm(true);
    }
  };

  const loadConfig = async () => {
    if (!selectedServer || !connectionData.username || !connectionData.password) {
      setError("Completa los datos de conexion SSH");
      return;
    }

    setIsLoading(true);
    setError(null);

    try {
      const response = await gatewayService.fetchRemoteConfig({
        hostname: connectionData.hostname,
        port: connectionData.port,
        username: connectionData.username,
        password: connectionData.password,
        config_path: connectionData.config_path,
      });

      if (response.success && response.config) {
        setConfigContent(response.config);
        // Save preferences
        localStorage.setItem(
          `codazzy-config-prefs-${selectedServer.node_id}`,
          JSON.stringify({
            hostname: connectionData.hostname,
            port: connectionData.port,
            username: connectionData.username,
            config_path: connectionData.config_path,
          })
        );
        setShowConnectionForm(false);
      } else {
        setError(response.error || "No se pudo cargar la configuracion");
      }
    } catch (err: any) {
      setError(err.message || "Error cargando configuracion");
    } finally {
      setIsLoading(false);
    }
  };

  const saveConfig = async () => {
    if (!selectedServer || !configContent) return;

    setIsSaving(true);
    setError(null);
    setSuccess(null);

    try {
      const response = await gatewayService.saveRemoteConfig({
        hostname: connectionData.hostname,
        port: connectionData.port,
        username: connectionData.username,
        password: connectionData.password,
        config_path: connectionData.config_path,
        config_content: configContent,
        restart_agent: true,
      });

      if (response.success) {
        setSuccess("Configuracion guardada y agente reiniciado");
      } else {
        setError(response.error || "Error guardando configuracion");
      }
    } catch (err: any) {
      setError(err.message || "Error guardando configuracion");
    } finally {
      setIsSaving(false);
    }
  };

  const inputClass =
    "w-full px-3 py-2 rounded-lg border border-[#2a3548] bg-[#0a0e17] text-[13px] text-white focus:outline-none focus:ring-2 focus:ring-emerald-500 placeholder-[#8b95a5]";
  const labelClass = "block text-[12px] text-[#8b95a5] uppercase tracking-wide mb-1.5";

  return (
    <div className="space-y-4">
      {isLoadingServers && (
        <div className="text-center py-8">
          <div className="text-[14px] text-[#8b95a5]">Cargando servidores...</div>
        </div>
      )}

      {!isLoadingServers && servers.length === 0 && (
        <div className="text-center py-8">
          <div className="text-4xl mb-3 opacity-50">&#128268;</div>
          <p className="text-[14px] text-[#8b95a5]">No hay servidores disponibles</p>
          <p className="text-[12px] text-[#5a6578] mt-1">
            Los servidores con agente instalado apareceran aqui automaticamente
          </p>
        </div>
      )}

      {!isLoadingServers && servers.length > 0 && !serverId && (
        <div>
          <label className={labelClass}>Seleccionar Servidor</label>
          <select
            value={selectedServer?.node_id || ""}
            onChange={(e) => {
              const server = servers.find((s) => s.node_id === e.target.value);
              if (server) handleSelectServer(server);
            }}
            className={inputClass}
          >
            <option value=""> Selecciona un dispositivo </option>
            {servers.map((server) => (
              <option key={server.node_id} value={server.node_id}>
                {server.node_id} ({server.hostname})
              </option>
            ))}
          </select>
        </div>
      )}

      {selectedServer && (
        <>
          <div className="p-3 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
            <div className="flex items-center justify-between mb-2">
              <span className="text-[14px] font-medium text-white">{selectedServer.node_id}</span>
            </div>
            <p className="text-[12px] text-[#8b95a5]">
              {selectedServer.hostname}
            </p>
            <div className="flex gap-3 mt-2 text-[11px] text-[#5a6578]">
              <span>OS: {selectedServer.os_type}</span>
              <span>Entorno: {selectedServer.environment}</span>
            </div>
          </div>

          {showConnectionForm && (
            <div className="space-y-3 p-3 rounded-lg bg-[#131a26] border border-[#2a3548]">
              <p className="text-[12px] text-[#8b95a5]">Datos de conexion SSH para cargar/guardar config:</p>
              <div className="grid grid-cols-3 gap-2">
                <div className="col-span-2">
                  <input
                    type="text"
                    value={connectionData.username}
                    onChange={(e) => setConnectionData({ ...connectionData, username: e.target.value })}
                    placeholder="Usuario SSH"
                    className={inputClass}
                  />
                </div>
                <div>
                  <input
                    type="number"
                    value={connectionData.port}
                    onChange={(e) => setConnectionData({ ...connectionData, port: parseInt(e.target.value) })}
                    placeholder="Puerto"
                    className={inputClass}
                  />
                </div>
              </div>
              <input
                type="password"
                value={connectionData.password}
                onChange={(e) => setConnectionData({ ...connectionData, password: e.target.value })}
                placeholder="Password SSH"
                className={inputClass}
              />
              <input
                type="text"
                value={connectionData.config_path}
                onChange={(e) => setConnectionData({ ...connectionData, config_path: e.target.value })}
                placeholder="Ruta config.toml"
                className={inputClass}
              />
            </div>
          )}

          <div className="flex gap-2">
            <button
              onClick={loadConfig}
              disabled={isLoading}
              className="flex-1 py-2 bg-[#1a2332] hover:bg-[#2a3548] text-white text-[13px] rounded-lg border border-[#2a3548] disabled:opacity-50 transition-colors"
            >
              {isLoading ? "Cargando..." : "Cargar Config"}
            </button>
            {!showConnectionForm && (
              <button
                onClick={() => setShowConnectionForm(true)}
                className="px-3 py-2 bg-[#1a2332] hover:bg-[#2a3548] text-white text-[13px] rounded-lg border border-[#2a3548] transition-colors"
              >
                Editar Conexion
              </button>
            )}
          </div>

          {configContent && (
            <div className="space-y-3">
              <label className={labelClass}>config.toml</label>
              <textarea
                value={configContent}
                onChange={(e) => setConfigContent(e.target.value)}
                rows={12}
                className={`${inputClass} font-mono text-[11px] resize-y`}
              />
              <button
                onClick={saveConfig}
                disabled={isSaving}
                className="w-full py-2.5 bg-emerald-500 hover:bg-emerald-600 text-[#0a0e17] text-[13px] font-medium rounded-lg disabled:opacity-50 transition-colors"
              >
                {isSaving ? "Guardando..." : "Guardar y Reiniciar Agente"}
              </button>
            </div>
          )}

          {error && (
            <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30">
              <p className="text-[13px] text-red-400">{error}</p>
            </div>
          )}
          {success && (
            <div className="p-3 rounded-lg bg-emerald-500/10 border border-emerald-500/30">
              <p className="text-[13px] text-emerald-400">{success}</p>
            </div>
          )}
        </>
      )}
    </div>
  );
}


