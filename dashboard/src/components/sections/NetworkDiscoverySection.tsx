"use client";

import { useRef, useReducer, useTransition } from "react";
import { Server, Router, Monitor, Settings, Save, X, Wifi, WifiOff } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  useNetworkDiscovery,
  getDeviceConfig,
  deviceTypeConfig,
} from "@/hooks/useNetworkDiscovery";
import type { DiscoveredDevice } from "@/types/discovery";
import {
  Button,
  Input,
  Select,
  Label,
  Badge,
  Textarea,
  ErrorBanner,
  EmptyState,
} from "@/components/ui/primitives";
import { Modal, ConfirmDialog } from "@/components/ui/Modal";

const DEVICE_ICONS: Record<string, typeof Server> = {
  server: Server,
  router: Router,
  switch: Wifi,
  unknown: Monitor,
};

function resolveIcon(type: string) {
  return DEVICE_ICONS[type] ?? Monitor;
}

type EditFormState = {
  hostname: string;
  device_type: string;
  description: string;
};

type EditFormAction =
  | { type: "SET_FIELD"; field: keyof EditFormState; value: string }
  | { type: "RESET"; payload: EditFormState };

function editFormReducer(state: EditFormState, action: EditFormAction): EditFormState {
  switch (action.type) {
    case "SET_FIELD":
      return { ...state, [action.field]: action.value };
    case "RESET":
      return action.payload;
  }
}

export function NetworkDiscoverySection() {
  const discovery = useNetworkDiscovery();
  const subnetRef = useRef<HTMLInputElement>(null);

  const [editModal, setEditModal] = useReducer(
    (_: DiscoveredDevice | null, next: DiscoveredDevice | null) => next,
    null
  );
  const [deleteTarget, setDeleteTarget] = useReducer(
    (_: DiscoveredDevice | null, next: DiscoveredDevice | null) => next,
    null
  );
  const [filterType, setFilterType] = useReducer((_: string, next: string) => next, "all");

  const filteredDevices =
    filterType === "all"
      ? discovery.devices
      : discovery.devices.filter((d) => d.device_type === filterType);

  const uniqueTypes = [...new Set(discovery.devices.map((d) => d.device_type))];

  function handleStartScan() {
    const target = subnetRef.current?.value ?? "192.168.1.0/24";
    discovery.startScan({ target_ranges: [target] });
  }

  function handleClearAll() {
    setDeleteTarget({ id: "__all__" } as DiscoveredDevice);
  }

  async function executeDelete() {
    if (!deleteTarget) return;

    if (deleteTarget.id === "__all__") {
      await discovery.clearDevices();
    } else {
      await discovery.deleteDevice(deleteTarget.id);
    }
    setDeleteTarget(null);
  }

  async function handleSaveEdit(updates: Partial<EditFormState>) {
    if (!editModal) return false;

    const success = await discovery.updateDevice(editModal.id, {
      hostname: updates.hostname,
      device_type: updates.device_type,
    });

    if (success) setEditModal(null);
    return success;
  }

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-12 gap-4">
        <ScanPanel
          ref={subnetRef}
          defaultSubnet="192.168.1.0/24"
          isScanning={discovery.isScanning}
          deviceCount={discovery.devices.length}
          onStartScan={handleStartScan}
          onStopScan={discovery.stopScan}
          onClearDevices={handleClearAll}
        />

        <NetworkMap
          devices={filteredDevices}
          selectedId={discovery.selectedDevice?.id ?? null}
          onSelect={discovery.selectDevice}
        />

        <DeviceList
          devices={filteredDevices}
          deviceTypes={uniqueTypes}
          selectedId={discovery.selectedDevice?.id ?? null}
          filterType={filterType}
          onSelect={discovery.selectDevice}
          onFilterChange={setFilterType}
        />
      </div>

      {discovery.error && (
        <ErrorBanner message={discovery.error} onDismiss={discovery.clearError} />
      )}

      {discovery.selectedDevice && (
        <DeviceDetails
          device={discovery.selectedDevice}
          onClose={() => discovery.selectDevice(null)}
          onEdit={() => setEditModal(discovery.selectedDevice)}
          onDelete={() => setDeleteTarget(discovery.selectedDevice)}
        />
      )}

      {editModal && (
        <EditDeviceModal
          device={editModal}
          onClose={() => setEditModal(null)}
          onSave={handleSaveEdit}
        />
      )}

      <ConfirmDialog
        open={deleteTarget !== null}
        onClose={() => setDeleteTarget(null)}
        onConfirm={executeDelete}
        title={deleteTarget?.id === "__all__" ? "Limpiar dispositivos" : "Eliminar dispositivo"}
        description={
          deleteTarget?.id === "__all__"
            ? "Se eliminarán todos los dispositivos descubiertos. Esta acción no se puede deshacer."
            : `¿Eliminar ${deleteTarget?.hostname || deleteTarget?.ip_address}?`
        }
        variant="danger"
        confirmText="Eliminar"
      />
    </div>
  );
}

interface ScanPanelProps {
  defaultSubnet: string;
  isScanning: boolean;
  deviceCount: number;
  onStartScan: () => void;
  onStopScan: () => void;
  onClearDevices: () => void;
}

const ScanPanel = ({
  ref,
  defaultSubnet,
  isScanning,
  deviceCount,
  onStartScan,
  onStopScan,
  onClearDevices,
}: ScanPanelProps & { ref: React.RefObject<HTMLInputElement | null> }) => (
  <div className="col-span-4 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
    <h4 className="text-[13px] font-medium text-white mb-3">Red a escanear</h4>

    <div className="space-y-3">
      <Input
        ref={ref}
        type="text"
        defaultValue={defaultSubnet}
        placeholder="192.168.1.0/24"
        disabled={isScanning}
      />

      <div className="flex gap-2">
        <Button
          variant="primary"
          size="md"
          onClick={onStartScan}
          disabled={isScanning}
          loading={isScanning}
          className="flex-1"
        >
          {isScanning ? "Escaneando..." : "Iniciar"}
        </Button>

        {isScanning && (
          <Button variant="danger" size="md" onClick={onStopScan}>
            Detener
          </Button>
        )}
      </div>

      <Button
        variant="secondary"
        size="md"
        onClick={onClearDevices}
        disabled={isScanning || deviceCount === 0}
        className="w-full"
      >
        Limpiar Dispositivos
      </Button>
    </div>
  </div>
);

interface NetworkMapProps {
  devices: DiscoveredDevice[];
  selectedId: string | null;
  onSelect: (d: DiscoveredDevice) => void;
}

function NetworkMap({ devices, selectedId, onSelect }: NetworkMapProps) {
  if (devices.length === 0) {
    return (
      <div className="col-span-5 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548] min-h-[400px]">
        <h4 className="text-[13px] font-medium text-white mb-3">Mapa de Red</h4>
        <EmptyState
          icon={<WifiOff className="w-10 h-10" />}
          title="Sin dispositivos"
          description="Inicia un escaneo para descubrir dispositivos en tu red"
          className="h-[350px]"
        />
      </div>
    );
  }

  return (
    <div className="col-span-5 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548] min-h-[400px]">
      <h4 className="text-[13px] font-medium text-white mb-3">Mapa de Red</h4>

      <div className="relative h-[350px] overflow-auto">
        <div className="flex flex-wrap gap-4 justify-center p-4">
          {devices.map((device) => {
            const config = getDeviceConfig(device.device_type);
            const Icon = resolveIcon(device.device_type);
            const isSelected = selectedId === device.id;

            return (
              <button
                key={device.id}
                type="button"
                onClick={() => onSelect(device)}
                className={cn(
                  "flex flex-col items-center transition-transform",
                  isSelected ? "scale-110" : "hover:scale-105"
                )}
              >
                <div
                  className={cn(
                    "w-12 h-12 rounded-full flex items-center justify-center shadow-lg",
                    config.color,
                    isSelected && "ring-2 ring-white ring-offset-2 ring-offset-[#0a0e17]"
                  )}
                >
                  <Icon className="w-5 h-5 text-white" />
                </div>
                <div className="mt-2 text-center bg-[#131a26]/90 px-2 py-1 rounded border border-[#2a3548]">
                  <p className="text-[10px] font-medium text-white truncate max-w-[80px]">
                    {device.hostname || device.ip_address}
                  </p>
                </div>
              </button>
            );
          })}
        </div>
      </div>
    </div>
  );
}

interface DeviceListProps {
  devices: DiscoveredDevice[];
  deviceTypes: string[];
  selectedId: string | null;
  filterType: string;
  onSelect: (d: DiscoveredDevice) => void;
  onFilterChange: (t: string) => void;
}

function DeviceList({
  devices,
  deviceTypes,
  selectedId,
  filterType,
  onSelect,
  onFilterChange,
}: DeviceListProps) {
  return (
    <div className="col-span-3 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
      <div className="flex items-center justify-between mb-3">
        <h4 className="text-[13px] font-medium text-white">
          Dispositivos ({devices.length})
        </h4>
      </div>

      <div className="space-y-2 mb-3">
        <Select
          value={filterType}
          onChange={(e) => onFilterChange(e.target.value)}
        >
          <option value="all">Todos los tipos</option>
          {deviceTypes.map((type) => (
            <option key={type} value={type}>
              {getDeviceConfig(type).label}
            </option>
          ))}
        </Select>
      </div>

      <div className="space-y-2 max-h-[350px] overflow-y-auto">
        {devices.length === 0 ? (
          <div className="text-center py-4 text-[11px] text-[#8b95a5]">
            No hay dispositivos
          </div>
        ) : (
          devices.map((device) => {
            const config = getDeviceConfig(device.device_type);
            const Icon = resolveIcon(device.device_type);
            const isSelected = selectedId === device.id;

            return (
              <button
                key={device.id}
                type="button"
                onClick={() => onSelect(device)}
                className={cn(
                  "w-full p-2 rounded-lg border text-left transition-all",
                  isSelected
                    ? "bg-emerald-500/10 border-emerald-500/50"
                    : "bg-[#131a26] border-[#2a3548] hover:border-[#3a4558]"
                )}
              >
                <div className="flex items-center gap-2">
                  <div
                    className={cn(
                      "w-8 h-8 rounded-full flex items-center justify-center flex-shrink-0",
                      config.color
                    )}
                  >
                    <Icon className="w-4 h-4 text-white" />
                  </div>
                  <div className="flex-1 min-w-0">
                    <p className="text-[11px] font-medium text-white truncate">
                      {device.hostname || device.ip_address}
                    </p>
                    <div className="flex items-center gap-2 text-[10px] text-[#8b95a5]">
                      <span>{device.ip_address}</span>
                      {device.status === "running" && (
                        <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />
                      )}
                    </div>
                  </div>
                  <Badge variant="default" size="xs">
                    {config.label}
                  </Badge>
                </div>
              </button>
            );
          })
        )}
      </div>
    </div>
  );
}

interface DeviceDetailsProps {
  device: DiscoveredDevice;
  onClose: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function DeviceDetails({ device, onClose, onEdit, onDelete }: DeviceDetailsProps) {
  const config = getDeviceConfig(device.device_type);
  const Icon = resolveIcon(device.device_type);

  return (
    <div className="p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div
            className={cn(
              "w-10 h-10 rounded-full flex items-center justify-center",
              config.color
            )}
          >
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <h4 className="text-[14px] font-medium text-white">
              {device.hostname || device.ip_address}
            </h4>
            <p className="text-[11px] text-[#8b95a5]">{config.label}</p>
          </div>
        </div>
        <button
          type="button"
          onClick={onClose}
          className="text-[#8b95a5] hover:text-white transition-colors"
        >
          <X className="w-5 h-5" />
        </button>
      </div>

      <dl className="grid grid-cols-2 gap-x-4 gap-y-2 text-[12px] mb-4">
        <dt className="text-[#8b95a5]">IP</dt>
        <dd className="text-white font-mono">{device.ip_address}</dd>

        {device.mac_address && (
          <>
            <dt className="text-[#8b95a5]">MAC</dt>
            <dd className="text-white font-mono">{device.mac_address}</dd>
          </>
        )}

        {device.vendor && (
          <>
            <dt className="text-[#8b95a5]">Fabricante</dt>
            <dd className="text-white">{device.vendor}</dd>
          </>
        )}

        {device.open_ports.length > 0 && (
          <>
            <dt className="text-[#8b95a5]">Puertos</dt>
            <dd className="text-white font-mono">
              {device.open_ports.slice(0, 5).join(", ")}
              {device.open_ports.length > 5 && ` +${device.open_ports.length - 5}`}
            </dd>
          </>
        )}
      </dl>

      <div className="flex gap-2 pt-4 border-t border-[#2a3548]">
        <Button variant="primary" size="sm" onClick={onEdit} className="flex-1">
          Editar
        </Button>
        <Button variant="danger" size="sm" onClick={onDelete}>
          Borrar
        </Button>
      </div>
    </div>
  );
}

interface EditDeviceModalProps {
  device: DiscoveredDevice;
  onClose: () => void;
  onSave: (updates: Partial<EditFormState>) => Promise<boolean>;
}

function EditDeviceModal({ device, onClose, onSave }: EditDeviceModalProps) {
  const [form, dispatch] = useReducer(editFormReducer, {
    hostname: device.hostname ?? "",
    device_type: device.device_type || "unknown",
    description: device.description ?? "",
  });
  const [isPending, startTransition] = useTransition();

  const config = getDeviceConfig(device.device_type);
  const Icon = resolveIcon(device.device_type);

  function handleSubmit() {
    startTransition(async () => {
      await onSave(form);
    });
  }

  return (
    <Modal
      open
      onClose={onClose}
      title="Editar Dispositivo"
      icon={<Settings className="w-5 h-5" />}
      footer={
        <>
          <Button variant="secondary" size="md" onClick={onClose} className="flex-1">
            Cancelar
          </Button>
          <Button
            variant="primary"
            size="md"
            onClick={handleSubmit}
            loading={isPending}
            className="flex-1"
          >
            <Save className="w-4 h-4" />
            Guardar
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex items-center gap-3 p-3 rounded-lg bg-[#0a0e17]">
          <div
            className={cn(
              "w-10 h-10 rounded-full flex items-center justify-center",
              config.color
            )}
          >
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <p className="text-[13px] font-medium text-white">{device.ip_address}</p>
            <p className="text-[11px] text-[#8b95a5]">
              {device.mac_address || "MAC no disponible"}
            </p>
          </div>
        </div>

        <div>
          <Label>Nombre</Label>
          <Input
            type="text"
            value={form.hostname}
            onChange={(e) =>
              dispatch({ type: "SET_FIELD", field: "hostname", value: e.target.value })
            }
            placeholder="Nombre del dispositivo"
          />
        </div>

        <div>
          <Label>Tipo de dispositivo</Label>
          <Select
            value={form.device_type}
            onChange={(e) =>
              dispatch({ type: "SET_FIELD", field: "device_type", value: e.target.value })
            }
          >
            {Object.entries(deviceTypeConfig).map(([key, cfg]) => (
              <option key={key} value={key}>
                {cfg.label}
              </option>
            ))}
          </Select>
        </div>

        <div>
          <Label>Descripción</Label>
          <Textarea
            value={form.description}
            onChange={(e) =>
              dispatch({ type: "SET_FIELD", field: "description", value: e.target.value })
            }
            placeholder="Notas sobre este dispositivo..."
            rows={3}
          />
        </div>
      </div>
    </Modal>
  );
}
