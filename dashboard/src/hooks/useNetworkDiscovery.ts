import { useState, useCallback, useRef, useEffect } from "react";
import {
  startScan as startScanAction,
  getScanStatus,
  stopScan as stopScanAction,
  getDiscoveredDevices,
  updateDevice as updateDeviceAction,
  deleteDevice as deleteDeviceAction,
  clearAllDevices,
  getNetworkTopology,
} from "@/app/actions/discovery";
import type { DiscoveredDevice, ScanStatusResponse, NetworkTopology, ScanConfiguration } from "@/types/discovery";

interface DiscoveryState {
  devices: DiscoveredDevice[];
  selectedDevice: DiscoveredDevice | null;
  topology: NetworkTopology | null;
  scanStatus: ScanStatusResponse | null;
  isScanning: boolean;
  isLoading: boolean;
  error: string | null;
}

export function useNetworkDiscovery() {
  const [state, setState] = useState<DiscoveryState>({
    devices: [],
    selectedDevice: null,
    topology: null,
    scanStatus: null,
    isScanning: false,
    isLoading: false,
    error: null,
  });

  const pollRef = useRef<NodeJS.Timeout | null>(null);
  const mountedRef = useRef(true);
  const currentScanId = useRef<string | null>(null);

  useEffect(() => {
    mountedRef.current = true;
    
    getDiscoveredDevices().then((data) => {
      if (mountedRef.current) {
        setState((s) => ({
          ...s,
          devices: (data.devices || []) as DiscoveredDevice[],
        }));
      }
    });

    return () => {
      mountedRef.current = false;
      if (pollRef.current) clearInterval(pollRef.current);
    };
  }, []);

  const setError = useCallback((error: string | null) => {
    if (mountedRef.current) setState((s) => ({ ...s, error }));
  }, []);

  const clearPoll = () => {
    if (pollRef.current) {
      clearInterval(pollRef.current);
      pollRef.current = null;
    }
  };

  const loadDevices = useCallback(async () => {
    setState((s) => ({ ...s, isLoading: true, error: null }));
    const data = await getDiscoveredDevices();
    if (mountedRef.current) {
      setState((s) => ({
        ...s,
        devices: (data.devices || []) as DiscoveredDevice[],
        isLoading: false,
      }));
    }
  }, []);

  const selectDevice = useCallback((device: DiscoveredDevice | null) => {
    setState((s) => ({ ...s, selectedDevice: device }));
  }, []);

  const updateDevice = useCallback(
    async (deviceId: string, updates: { hostname?: string; device_type?: string }) => {
      const result = await updateDeviceAction(deviceId, updates);
      if (result.success && mountedRef.current) {
        setState((s) => ({
          ...s,
          devices: s.devices.map((d) => (d.id === deviceId ? { ...d, ...updates } : d)),
          selectedDevice: s.selectedDevice?.id === deviceId ? { ...s.selectedDevice, ...updates } : s.selectedDevice,
        }));
      } else if (!result.success) {
        setError(result.error || "Error actualizando dispositivo");
      }
      return result.success;
    },
    [setError]
  );

  const deleteDevice = useCallback(
    async (deviceId: string) => {
      const result = await deleteDeviceAction(deviceId);
      if (result.success && mountedRef.current) {
        setState((s) => ({
          ...s,
          devices: s.devices.filter((d) => d.id !== deviceId),
          selectedDevice: s.selectedDevice?.id === deviceId ? null : s.selectedDevice,
        }));
      } else if (!result.success) {
        setError(result.error || "Error eliminando dispositivo");
      }
      return result.success;
    },
    [setError]
  );

  const loadTopology = useCallback(async () => {
    const data = await getNetworkTopology();
    if (mountedRef.current) {
      setState((s) => ({ ...s, topology: data as NetworkTopology }));
    }
  }, []);

  const pollScanStatus = useCallback(
    async (scanId: string) => {
      const status = await getScanStatus(scanId);
      if (!mountedRef.current) return;

      if (status.error) {
        clearPoll();
        setState((s) => ({ ...s, isScanning: false, error: status.error || null }));
        return;
      }

      setState((s) => ({ ...s, scanStatus: status as ScanStatusResponse }));

      if (status.status === "completed") {
        clearPoll();
        setState((s) => ({ ...s, isScanning: false }));
        await loadDevices();
        await loadTopology();
      } else if (status.status === "failed" || status.status === "cancelled") {
        clearPoll();
        setState((s) => ({
          ...s,
          isScanning: false,
          error: status.error || "Escaneo cancelado",
        }));
      }
    },
    [loadDevices, loadTopology]
  );

  const startScan = useCallback(
    async (config: ScanConfiguration) => {
      setState((s) => ({ ...s, isScanning: true, error: null, scanStatus: null }));

      const result = await startScanAction({
        target: config.target_ranges?.[0] || "",
        scan_type: config.protocols?.includes("snmp") ? "full" : "icmp",
        timeout_seconds: config.options?.timeout,
      });

      if (!result.success || !result.job_id) {
        if (mountedRef.current) {
          setState((s) => ({
            ...s,
            isScanning: false,
            error: result.error || "Error iniciando escaneo",
          }));
        }
        return null;
      }

      currentScanId.current = result.job_id;
      pollRef.current = setInterval(() => pollScanStatus(result.job_id!), 500);
      pollScanStatus(result.job_id);

      return { scan_id: result.job_id };
    },
    [pollScanStatus]
  );

  const stopScan = useCallback(async (scanId?: string) => {
    const id = scanId || currentScanId.current;
    if (!id) return;

    await stopScanAction(id);
    clearPoll();
    setState((s) => ({ ...s, isScanning: false }));
  }, []);

  const clearDevices = useCallback(async () => {
    const result = await clearAllDevices();
    if (result.success && mountedRef.current) {
      setState((s) => ({ ...s, devices: [], selectedDevice: null, topology: null }));
    } else if (!result.success) {
      setError(result.error || "Error limpiando dispositivos");
    }
    return result.success;
  }, [setError]);

  return {
    devices: state.devices,
    selectedDevice: state.selectedDevice,
    topology: state.topology,
    scanStatus: state.scanStatus,
    isScanning: state.isScanning,
    isLoading: state.isLoading,
    error: state.error,
    loadDevices,
    selectDevice,
    updateDevice,
    deleteDevice,
    clearDevices,
    startScan,
    stopScan,
    loadTopology,
    clearError: () => setError(null),
  };
}

export const deviceTypeConfig: Record<string, { color: string; label: string }> = {
  server: { color: "bg-blue-500", label: "Server" },
  router: { color: "bg-purple-500", label: "Router" },
  gateway: { color: "bg-purple-600", label: "Gateway" },
  switch: { color: "bg-green-500", label: "Switch" },
  database: { color: "bg-orange-500", label: "Database" },
  "api-gateway": { color: "bg-cyan-500", label: "API Gateway" },
  "message-queue": { color: "bg-yellow-500", label: "Message Queue" },
  container: { color: "bg-indigo-500", label: "Container" },
  unknown: { color: "bg-gray-500", label: "Unknown" },
};

export function getDeviceConfig(type: string) {
  return deviceTypeConfig[type] || deviceTypeConfig.unknown;
}
