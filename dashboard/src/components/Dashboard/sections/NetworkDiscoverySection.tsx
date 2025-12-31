/**
 * NetworkDiscoverySection - Refactored with simplified hook
 */

"use client";

import { useEffect, useState, useMemo } from "react";
import { Server, Router, Monitor, Settings, Save, X } from "lucide-react";
import { cn } from "@/lib/utils";
import {
  useNetworkDiscovery,
  getDeviceConfig,
  deviceTypeConfig,
} from "@/hooks/useNetworkDiscovery";
import type { DiscoveredDevice } from "@/types/discovery";
import { Button, Input, Select, Label, Badge } from "@/components/ui/primitives";
import { Modal, ConfirmDialog } from "@/components/ui/Modal";

// ============================================================================
// Main Component
// ============================================================================

export function NetworkDiscoverySection() {
  const discovery = useNetworkDiscovery();
  const [subnet, setSubnet] = useState("192.168.1.0/24");
  const [editModal, setEditModal] = useState<DiscoveredDevice | null>(null);
  const [deleteConfirm, setDeleteConfirm] = useState(false);
  const [filterType, setFilterType] = useState("all");

  useEffect(() => {
    discovery.loadDevices();
  }, [discovery.loadDevices]);

  const handleStartScan = () => discovery.startScan({ target_ranges: [subnet] });
  const handleClearDevices = async () => {
    if (!confirm("¿Eliminar todos los dispositivos descubiertos?")) return;
    await discovery.clearDevices();
  };

  // Filtered devices
  const filteredDevices = useMemo(() => {
    if (filterType === "all") return discovery.devices;
    return discovery.devices.filter(d => d.device_type === filterType);
  }, [discovery.devices, filterType]);

  // Device types for filter
  const deviceTypes = useMemo(() => {
    return Array.from(new Set(discovery.devices.map(d => d.device_type)));
  }, [discovery.devices]);

  return (
    <div className="space-y-4">
      <div className="grid grid-cols-12 gap-4">
        {/* Scan Controls */}
        <ScanPanel
          subnet={subnet}
          onSubnetChange={setSubnet}
          isScanning={discovery.isScanning}
          deviceCount={discovery.devices.length}
          onStartScan={handleStartScan}
          onStopScan={discovery.stopScan}
          onClearDevices={handleClearDevices}
        />

        {/* Network Map */}
        <NetworkMap
          devices={filteredDevices}
          selectedDevice={discovery.selectedDevice}
          onSelectDevice={discovery.selectDevice}
        />

        {/* Device List */}
        <DeviceList
          devices={filteredDevices}
          deviceTypes={deviceTypes}
          selectedDevice={discovery.selectedDevice}
          filterType={filterType}
          onSelectDevice={discovery.selectDevice}
          onFilterChange={setFilterType}
        />
      </div>

      {discovery.error && (
        <div className="p-3 rounded-lg bg-red-500/10 border border-red-500/30 text-red-400 text-sm">
          {discovery.error}
        </div>
      )}

      {/* Selected Device Details */}
      {discovery.selectedDevice && (
        <DeviceDetails
          device={discovery.selectedDevice}
          onClose={() => discovery.selectDevice(null)}
          onEdit={() => setEditModal(discovery.selectedDevice)}
          onDelete={() => setDeleteConfirm(true)}
        />
      )}

      {/* Edit Modal */}
      {editModal && (
        <EditDeviceModal
          device={editModal}
          onClose={() => setEditModal(null)}
          onSave={async updates => {
            const success = await discovery.updateDevice(editModal.id, updates);
            if (success) setEditModal(null);
          }}
        />
      )}

      {/* Delete Confirmation */}
      <ConfirmDialog
        open={deleteConfirm && !!discovery.selectedDevice}
        onClose={() => setDeleteConfirm(false)}
        onConfirm={async () => {
          if (discovery.selectedDevice) {
            await discovery.deleteDevice(discovery.selectedDevice.id);
          }
          setDeleteConfirm(false);
        }}
        title="Eliminar dispositivo"
        description="¿Estás seguro de que quieres eliminar este dispositivo?"
        variant="danger"
        confirmText="Eliminar"
      />
    </div>
  );
}

// ============================================================================
// Scan Panel
// ============================================================================

interface ScanPanelProps {
  subnet: string;
  onSubnetChange: (v: string) => void;
  isScanning: boolean;
  deviceCount: number;
  onStartScan: () => void;
  onStopScan: () => void;
  onClearDevices: () => void;
}

function ScanPanel({
  subnet,
  onSubnetChange,
  isScanning,
  deviceCount,
  onStartScan,
  onStopScan,
  onClearDevices,
}: ScanPanelProps) {
  return (
    <div className="col-span-4 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
      <h4 className="text-[13px] font-medium text-white mb-3">Red a escanear</h4>

      <div className="space-y-3">
        <Input
          type="text"
          value={subnet}
          onChange={e => onSubnetChange(e.target.value)}
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
}

// ============================================================================
// Network Map
// ============================================================================

interface NetworkMapProps {
  devices: DiscoveredDevice[];
  selectedDevice: DiscoveredDevice | null;
  onSelectDevice: (d: DiscoveredDevice) => void;
}

function NetworkMap({ devices, selectedDevice, onSelectDevice }: NetworkMapProps) {
  if (devices.length === 0) {
    return (
      <div className="col-span-5 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548] min-h-[400px]">
        <h4 className="text-[13px] font-medium text-white mb-3">Mapa de Red</h4>
        <div className="h-[350px] flex items-center justify-center">
          <p className="text-[12px] text-[#8b95a5]">No hay dispositivos en el mapa</p>
        </div>
      </div>
    );
  }

  return (
    <div className="col-span-5 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548] min-h-[400px]">
      <h4 className="text-[13px] font-medium text-white mb-3">Mapa de Red</h4>

      <div className="relative h-[350px] overflow-auto">
        <div className="flex flex-wrap gap-4 justify-center p-4">
          {devices.map(device => (
            <DeviceNode
              key={device.id}
              device={device}
              isSelected={selectedDevice?.id === device.id}
              onClick={() => onSelectDevice(device)}
            />
          ))}
        </div>
      </div>
    </div>
  );
}

function DeviceNode({
  device,
  isSelected,
  onClick,
}: {
  device: DiscoveredDevice;
  isSelected: boolean;
  onClick: () => void;
}) {
  const config = getDeviceConfig(device.device_type);
  const Icon = getDeviceIcon(device.device_type);

  return (
    <div
      onClick={onClick}
      className={cn(
        "flex flex-col items-center cursor-pointer transition-all",
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
    </div>
  );
}

// ============================================================================
// Device List
// ============================================================================

interface DeviceListProps {
  devices: DiscoveredDevice[];
  deviceTypes: string[];
  selectedDevice: DiscoveredDevice | null;
  filterType: string;
  onSelectDevice: (d: DiscoveredDevice) => void;
  onFilterChange: (t: string) => void;
}

function DeviceList({
  devices,
  deviceTypes,
  selectedDevice,
  filterType,
  onSelectDevice,
  onFilterChange,
}: DeviceListProps) {
  return (
    <div className="col-span-3 p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
      <div className="flex items-center justify-between mb-3">
        <h4 className="text-[13px] font-medium text-white">Dispositivos ({devices.length})</h4>
      </div>

      <div className="space-y-2 mb-3">
        <Select value={filterType} onChange={e => onFilterChange(e.target.value)}>
          <option value="all">Todos los tipos</option>
          {deviceTypes.map(type => (
            <option key={type} value={type}>
              {getDeviceConfig(type).label}
            </option>
          ))}
        </Select>
      </div>

      <div className="space-y-2 max-h-[350px] overflow-y-auto">
        {devices.length === 0 ? (
          <div className="text-center py-4 text-[11px] text-[#8b95a5]">No hay dispositivos</div>
        ) : (
          devices.map(device => (
            <DeviceListItem
              key={device.id}
              device={device}
              isSelected={selectedDevice?.id === device.id}
              onClick={() => onSelectDevice(device)}
            />
          ))
        )}
      </div>
    </div>
  );
}

function DeviceListItem({
  device,
  isSelected,
  onClick,
}: {
  device: DiscoveredDevice;
  isSelected: boolean;
  onClick: () => void;
}) {
  const config = getDeviceConfig(device.device_type);
  const Icon = getDeviceIcon(device.device_type);

  return (
    <div
      onClick={onClick}
      className={cn(
        "p-2 rounded-lg border cursor-pointer transition-all",
        isSelected
          ? "bg-emerald-500/10 border-emerald-500/50"
          : "bg-[#131a26] border-[#2a3548] hover:border-[#3a4558]"
      )}
    >
      <div className="flex items-center gap-2">
        <div className={cn("w-8 h-8 rounded-full flex items-center justify-center", config.color)}>
          <Icon className="w-4 h-4 text-white" />
        </div>
        <div className="flex-1 min-w-0">
          <p className="text-[11px] font-medium text-white truncate">
            {device.hostname || device.ip_address}
          </p>
          <div className="flex items-center gap-2 text-[10px] text-[#8b95a5]">
            <span>{device.ip_address}</span>
            {device.status === "running" && <span className="w-1.5 h-1.5 rounded-full bg-emerald-500" />}
          </div>
        </div>
        <Badge variant="default" size="xs">
          {config.label}
        </Badge>
      </div>
    </div>
  );
}

// ============================================================================
// Device Details
// ============================================================================

interface DeviceDetailsProps {
  device: DiscoveredDevice;
  onClose: () => void;
  onEdit: () => void;
  onDelete: () => void;
}

function DeviceDetails({ device, onClose, onEdit, onDelete }: DeviceDetailsProps) {
  const config = getDeviceConfig(device.device_type);
  const Icon = getDeviceIcon(device.device_type);

  return (
    <div className="p-4 rounded-lg bg-[#0a0e17] border border-[#2a3548]">
      <div className="flex items-center justify-between mb-3">
        <div className="flex items-center gap-3">
          <div className={cn("w-10 h-10 rounded-full flex items-center justify-center", config.color)}>
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <h4 className="text-[14px] font-medium text-white">
              {device.hostname || device.ip_address}
            </h4>
            <p className="text-[11px] text-[#8b95a5]">{config.label}</p>
          </div>
        </div>
        <button onClick={onClose} className="text-[#8b95a5] hover:text-white transition-colors">
          <X className="w-5 h-5" />
        </button>
      </div>

      <div className="flex gap-2 mt-4 pt-4 border-t border-[#2a3548]">
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

// ============================================================================
// Edit Device Modal
// ============================================================================

interface EditDeviceModalProps {
  device: DiscoveredDevice;
  onClose: () => void;
  onSave: (updates: { hostname?: string; device_type?: string; description?: string }) => Promise<void>;
}

function EditDeviceModal({ device, onClose, onSave }: EditDeviceModalProps) {
  const [form, setForm] = useState({
    hostname: device.hostname || "",
    device_type: device.device_type || "unknown",
    description: device.description || "",
  });
  const [saving, setSaving] = useState(false);

  const handleSave = async () => {
    setSaving(true);
    await onSave(form);
    setSaving(false);
  };

  const config = getDeviceConfig(device.device_type);
  const Icon = getDeviceIcon(device.device_type);

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
          <Button variant="primary" size="md" onClick={handleSave} loading={saving} className="flex-1">
            <Save className="w-4 h-4" />
            Guardar Cambios
          </Button>
        </>
      }
    >
      <div className="space-y-4">
        <div className="flex items-center gap-3 p-3 rounded-lg bg-[#0a0e17]">
          <div className={cn("w-10 h-10 rounded-full flex items-center justify-center", config.color)}>
            <Icon className="w-5 h-5 text-white" />
          </div>
          <div>
            <p className="text-[13px] font-medium text-white">{device.ip_address}</p>
            <p className="text-[11px] text-[#8b95a5]">{device.mac_address || "MAC no disponible"}</p>
          </div>
        </div>

        <div>
          <Label>Nombre</Label>
          <Input
            type="text"
            value={form.hostname}
            onChange={e => setForm(f => ({ ...f, hostname: e.target.value }))}
            placeholder="Nombre"
          />
        </div>

        <div>
          <Label>Tipo de dispositivo</Label>
          <Select
            value={form.device_type}
            onChange={e => setForm(f => ({ ...f, device_type: e.target.value }))}
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
          <textarea
            value={form.description}
            onChange={e => setForm(f => ({ ...f, description: e.target.value }))}
            placeholder="Notas sobre este dispositivo..."
            rows={3}
            className="w-full px-3 py-2 bg-[#0a0e17] border border-[#2a3548] rounded-lg text-[13px] text-white placeholder-[#5a6577] focus:outline-none focus:border-emerald-500 resize-none"
          />
        </div>
      </div>
    </Modal>
  );
}

// ============================================================================
// Icon Helper
// ============================================================================

function getDeviceIcon(type: string) {
  switch (type) {
    case "server":
      return Server;
    case "router":
      return Router;
    default:
      return Monitor;
  }
}
