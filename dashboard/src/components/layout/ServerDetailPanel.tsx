"use client";

import { useEffect } from "react";
import { cn } from "@/lib/utils";
import { SidePanel, PanelTabs } from "./SidePanel";
import { useServerPanel, formatFileSize } from "@/hooks/useServerPanel";
import { Button, Input, Textarea, Label, Badge, Spinner, ErrorBanner, ProgressBar } from "@/components/ui/primitives";
import type { Agent, ServerConnection, ServerDocument } from "@/types/gateway";


interface ServerDetailPanelProps {
  agent: Agent;
  connection?: ServerConnection;
  activeTab: string;
  onTabChange: (tab: string) => void;
  onClose: () => void;
  onGenerateReport: () => void;
  onOpenConfig: () => void;
}


export function ServerDetailPanel({
  agent,
  connection,
  activeTab,
  onTabChange,
  onClose,
  onGenerateReport,
  onOpenConfig,
}: ServerDetailPanelProps) {
  const panel = useServerPanel(agent.id);

  useEffect(() => {
    panel.loadData();
  }, [agent.id, panel.loadData]);

  return (
    <SidePanel isOpen onClose={onClose} title={agent.name || agent.id}>
      <PanelTabs
        tabs={["Resumen", "Contexto", "Documentos"]}
        activeTab={activeTab}
        onChange={onTabChange}
      />

      {panel.error && <ErrorBanner message={panel.error} className="mb-4" />}

      {activeTab === "Resumen" && (
        <SummaryTab
          agent={agent}
          connection={connection}
          onOpenConfig={onOpenConfig}
          onGenerateReport={onGenerateReport}
        />
      )}

      {activeTab === "Contexto" && (
        <ContextTab
          description={panel.description}
          notes={panel.notes}
          saving={panel.saving}
          onDescriptionChange={panel.setDescription}
          onNotesChange={panel.setNotes}
          onSave={panel.saveMeta}
        />
      )}

      {activeTab === "Documentos" && (
        <DocumentsTab
          documents={panel.documents}
          loading={panel.loadingDocs}
          uploading={panel.uploading}
          dragActive={panel.dragActive}
          allowedExtensions={panel.allowedExtensions}
          onDragEnter={panel.handleDragEnter}
          onDragLeave={panel.handleDragLeave}
          onDragOver={panel.handleDragOver}
          onDrop={panel.handleDrop}
          onFileSelect={panel.uploadFiles}
          onRemove={panel.removeDocument}
        />
      )}
    </SidePanel>
  );
}


interface SummaryTabProps {
  agent: Agent;
  connection?: ServerConnection;
  onOpenConfig: () => void;
  onGenerateReport: () => void;
}

function SummaryTab({ agent, connection, onOpenConfig, onGenerateReport }: SummaryTabProps) {
  return (
    <div className="space-y-5">
      <InfoSection agent={agent} connection={connection} />
      <MetricsSection agent={agent} />
      <ActionsSection onOpenConfig={onOpenConfig} onGenerateReport={onGenerateReport} />
    </div>
  );
}

function InfoSection({ agent, connection }: { agent: Agent; connection?: ServerConnection }) {
  const rows = [
    { label: "ID", value: agent.id, mono: true },
    { label: "Tipo", value: agent.type || "Server" },
    { label: "Ubicacion", value: agent.location || connection?.location || "Datacenter-A" },
    connection && { label: "Entorno", value: connection.environment, capitalize: true },
    connection && { label: "SO", value: connection.os_type, capitalize: true },
  ].filter(Boolean) as { label: string; value: string; mono?: boolean; capitalize?: boolean }[];

  return (
    <section>
      <SectionHeader>Informacion</SectionHeader>
      <dl className="bg-[var(--color-bg-primary)] rounded-lg p-3 space-y-2 text-[13px]">
        {rows.map(({ label, value, mono, capitalize }) => (
          <div key={label} className="flex justify-between">
            <dt className="text-[var(--color-text-secondary)]">{label}</dt>
            <dd className={cn(mono && "font-mono", capitalize && "capitalize")}>{value}</dd>
          </div>
        ))}
      </dl>
    </section>
  );
}

function MetricsSection({ agent }: { agent: Agent }) {
  return (
    <section>
      <SectionHeader>Metricas Actuales</SectionHeader>
      <div className="space-y-2">
        <MetricRow label="CPU" value={agent.cpu_usage} variant="cpu" />
        <MetricRow label="RAM" value={agent.memory_usage} variant="memory" />
      </div>
    </section>
  );
}

function MetricRow({ label, value, variant }: { label: string; value: number; variant: "cpu" | "memory" }) {
  return (
    <div className="flex items-center gap-3">
      <span className="text-[12px] text-[var(--color-text-secondary)] w-9">{label}</span>
      <ProgressBar value={value} variant={variant} showLabel className="flex-1" />
    </div>
  );
}

function ActionsSection({ onOpenConfig, onGenerateReport }: { onOpenConfig: () => void; onGenerateReport: () => void }) {
  return (
    <div className="flex gap-2 pt-4">
      <Button variant="primary" size="md" onClick={onOpenConfig} className="flex-1">
        Configuracion de Agente
      </Button>
      <Button
        variant="secondary"
        size="md"
        onClick={onGenerateReport}
        className="flex-1 bg-[var(--color-purple)] hover:opacity-90 text-white border-0"
      >
        Generar Informe
      </Button>
    </div>
  );
}


interface ContextTabProps {
  description: string;
  notes: string;
  saving: boolean;
  onDescriptionChange: (v: string) => void;
  onNotesChange: (v: string) => void;
  onSave: () => void;
}

function ContextTab({ description, notes, saving, onDescriptionChange, onNotesChange, onSave }: ContextTabProps) {
  return (
    <div className="space-y-4">
      <div>
        <Label>Descripción</Label>
        <Input
          type="text"
          value={description}
          onChange={e => onDescriptionChange(e.target.value)}
          placeholder="Descripción breve"
        />
      </div>

      <div>
        <Label>Notas adicionales</Label>
        <Textarea
          value={notes}
          onChange={e => onNotesChange(e.target.value)}
          placeholder="Añade toda la información relevante sobre este dispositivo"
          rows={4}
        />
      </div>

      <Button variant="primary" size="md" onClick={onSave} loading={saving} className="w-full">
        {saving ? "Guardando..." : "Guardar"}
      </Button>
    </div>
  );
}


interface DocumentsTabProps {
  documents: ServerDocument[];
  loading: boolean;
  uploading: boolean;
  dragActive: boolean;
  allowedExtensions: string[];
  onDragEnter: (e: React.DragEvent) => void;
  onDragLeave: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  onFileSelect: (files: FileList | null) => void;
  onRemove: (id: number) => void;
}

function DocumentsTab({
  documents,
  loading,
  uploading,
  dragActive,
  allowedExtensions,
  onDragEnter,
  onDragLeave,
  onDragOver,
  onDrop,
  onFileSelect,
  onRemove,
}: DocumentsTabProps) {
  return (
    <div className="space-y-4">
      <p className="text-[12px] text-[var(--color-text-secondary)]">
        Sube Documentos sobre el dispositivo como contexto (formato ligero).
      </p>

      <DropZone
        active={dragActive}
        uploading={uploading}
        extensions={allowedExtensions}
        onDragEnter={onDragEnter}
        onDragLeave={onDragLeave}
        onDragOver={onDragOver}
        onDrop={onDrop}
        onFileSelect={onFileSelect}
      />

      <DocumentList documents={documents} loading={loading} onRemove={onRemove} />
    </div>
  );
}

interface DropZoneProps {
  active: boolean;
  uploading: boolean;
  extensions: string[];
  onDragEnter: (e: React.DragEvent) => void;
  onDragLeave: (e: React.DragEvent) => void;
  onDragOver: (e: React.DragEvent) => void;
  onDrop: (e: React.DragEvent) => void;
  onFileSelect: (files: FileList | null) => void;
}

function DropZone({ active, uploading, extensions, onDragEnter, onDragLeave, onDragOver, onDrop, onFileSelect }: DropZoneProps) {
  return (
    <div
      onDragEnter={onDragEnter}
      onDragLeave={onDragLeave}
      onDragOver={onDragOver}
      onDrop={onDrop}
      className={cn(
        "relative border-2 border-dashed rounded-lg p-6 text-center transition-colors",
        active && "border-[var(--color-accent)] bg-[var(--color-accent)]/10",
        uploading && "border-[var(--color-warning)] bg-[var(--color-warning)]/10",
        !active && !uploading && "border-[var(--color-border)] hover:border-[var(--color-border-hover)]"
      )}
    >
      <input
        type="file"
        multiple
        accept={extensions.join(",")}
        onChange={e => onFileSelect(e.target.files)}
        className="absolute inset-0 w-full h-full opacity-0 cursor-pointer"
        disabled={uploading}
      />
      <div className="text-[var(--color-text-secondary)]">
        <UploadIcon />
        <p className="text-[13px] font-medium">{uploading ? "Subiendo..." : "Arrastra o clic"}</p>
        <p className="text-[11px] mt-1">txt, json, yaml (1MB)</p>
      </div>
    </div>
  );
}

function DocumentList({ documents, loading, onRemove }: { documents: ServerDocument[]; loading: boolean; onRemove: (id: number) => void }) {
  if (loading) {
    return (
      <div className="text-center py-4">
        <Spinner size="sm" />
        <span className="text-[var(--color-text-secondary)] text-[13px] ml-2">Cargando...</span>
      </div>
    );
  }

  if (documents.length === 0) {
    return (
      <div className="text-center text-[var(--color-text-secondary)] text-[13px] py-4">
        Sin documentos
      </div>
    );
  }

  return (
    <ul className="space-y-2">
      {documents.map(doc => (
        <DocumentItem key={doc.id} doc={doc} onRemove={onRemove} />
      ))}
    </ul>
  );
}

function DocumentItem({ doc, onRemove }: { doc: ServerDocument; onRemove: (id: number) => void }) {
  return (
    <li className="flex items-center justify-between p-3 bg-[var(--color-bg-primary)] rounded-lg">
      <div className="flex items-center gap-3 min-w-0">
        <FileIcon />
        <div className="min-w-0">
          <p className="text-[13px] font-medium text-white truncate">{doc.filename}</p>
          <p className="text-[11px] text-[var(--color-text-secondary)]">
            {formatFileSize(doc.file_size)} - {new Date(doc.created_at).toLocaleDateString()}
          </p>
        </div>
      </div>
      <button
        onClick={() => onRemove(doc.id)}
        className="p-1.5 text-[var(--color-text-secondary)] hover:text-[var(--color-error)] hover:bg-[var(--color-error)]/10 rounded transition-colors"
      >
        <TrashIcon />
      </button>
    </li>
  );
}


function SectionHeader({ children }: { children: React.ReactNode }) {
  return (
    <h3 className="text-[12px] text-[var(--color-text-secondary)] uppercase tracking-wide mb-2">
      {children}
    </h3>
  );
}


function UploadIcon() {
  return (
    <svg className="w-8 h-8 mx-auto mb-2" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4" />
      <polyline points="17 8 12 3 7 8" />
      <line x1="12" y1="3" x2="12" y2="15" />
    </svg>
  );
}

function FileIcon() {
  return (
    <svg className="w-5 h-5 text-[var(--color-text-secondary)]" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <path d="M14 2H6a2 2 0 0 0-2 2v16a2 2 0 0 0 2 2h12a2 2 0 0 0 2-2V8z" />
      <polyline points="14 2 14 8 20 8" />
    </svg>
  );
}

function TrashIcon() {
  return (
    <svg className="w-4 h-4" viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth="2">
      <polyline points="3 6 5 6 21 6" />
      <path d="M19 6v14a2 2 0 0 1-2 2H7a2 2 0 0 1-2-2V6m3 0V4a2 2 0 0 1 2-2h4a2 2 0 0 1 2 2v2" />
    </svg>
  );
}
