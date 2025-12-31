"use client";

import { useEffect, useRef } from "react";
import { FileText, Download, ChevronDown, ChevronUp, BarChart3, Activity, Network } from "lucide-react";
import ReactMarkdown from "react-markdown";
import remarkGfm from "remark-gfm";
import { cn } from "@/lib/utils";
import {
  useAIReport,
  REPORT_TYPES,
  getMetricValue,
  type Agent,
  type GeneratedReport,
  type ReportConfig,
} from "@/hooks/useAIReport";
import { exportToPDF, exportToMarkdown, exportDebugPrompt } from "@/lib/pdf-export";
import { Button, Input, Select, Label, Badge, ErrorBanner, Card } from "@/components/ui/primitives";


interface AIReportSectionProps {
  agents?: Agent[];
}

export function AIReportSection({ agents = [] }: AIReportSectionProps) {
  const report = useAIReport(agents);
  const prevAgentsRef = useRef<string>("");

  useEffect(() => {
    const agentIds = agents.map(a => a.id).join(",");
    if (agentIds === prevAgentsRef.current) return;
    prevAgentsRef.current = agentIds;
    report.loadServers();
  }, [agents, report.loadServers]);

  const currentReportType = REPORT_TYPES.find(t => t.id === report.config.type);

  return (
    <div className="space-y-4">
      <ReportTypeSelector
        selectedType={report.config.type}
        onSelect={type => report.updateConfig({ type: type as ReportConfig["type"] })}
      />

      <div>
        <Label>Titulo del Informe</Label>
        <Input
          type="text"
          value={report.config.title}
          onChange={e => report.updateConfig({ title: e.target.value })}
          placeholder={`Informe ${currentReportType?.name} - ${new Date().toLocaleDateString()}`}
        />
      </div>

      {currentReportType?.selectServers !== false && (
        <ServerSelector
          servers={report.servers}
          selectedServers={report.config.servers}
          onToggle={report.toggleServer}
          onSelectAll={report.selectAllServers}
          onDeselectAll={report.deselectAllServers}
        />
      )}

      <ConfigOptions
        config={report.config}
        onUpdate={report.updateConfig}
      />

      <Button
        variant="primary"
        size="lg"
        onClick={report.generateReport}
        disabled={report.isGenerating || report.config.servers.length === 0}
        loading={report.isGenerating}
        className="w-full"
      >
        <FileText className="w-4 h-4" />
        {report.isGenerating ? "Generando..." : "Generar Informe"}
      </Button>

      {report.error && <ErrorBanner message={report.error} />}

      {report.generatedReports.length > 0 && (
        <HistoryToggle
          count={report.generatedReports.length}
          isOpen={report.showHistory}
          onToggle={report.toggleHistory}
        />
      )}

      {report.showHistory && report.generatedReports.length > 0 && (
        <ReportHistory
          reports={report.generatedReports}
          selectedReport={report.selectedReport}
          onSelect={report.selectReport}
        />
      )}

      {report.selectedReport && (
        <ReportPreview report={report.selectedReport} />
      )}
    </div>
  );
}


interface ReportTypeSelectorProps {
  selectedType: string;
  onSelect: (type: string) => void;
}

function ReportTypeSelector({ selectedType, onSelect }: ReportTypeSelectorProps) {
  const icons = {
    executive: BarChart3,
    technical: Activity,
    network_performance: Network,
  };

  return (
    <div>
      <Label>Tipo de Informe</Label>
      <div className="grid grid-cols-3 gap-2">
        {REPORT_TYPES.map(type => {
          const Icon = icons[type.id as keyof typeof icons];
          const isSelected = selectedType === type.id;

          return (
            <button
              key={type.id}
              onClick={() => onSelect(type.id)}
              className={cn(
                "p-3 rounded-lg border text-left transition-colors",
                isSelected
                  ? "border-emerald-500 bg-emerald-500/10"
                  : "border-[#2a3548] bg-[#0a0e17] hover:border-[#3a4558]"
              )}
            >
              <div className="flex items-center gap-2 mb-1">
                <Icon className="w-4 h-4 text-emerald-400" />
                <span className="text-[13px] font-medium text-white">{type.name}</span>
              </div>
              <div className="text-[11px] text-[#8b95a5]">{type.description}</div>
            </button>
          );
        })}
      </div>
    </div>
  );
}


interface ServerSelectorProps {
  servers: string[];
  selectedServers: string[];
  onToggle: (id: string) => void;
  onSelectAll: () => void;
  onDeselectAll: () => void;
}

function ServerSelector({ servers, selectedServers, onToggle, onSelectAll, onDeselectAll }: ServerSelectorProps) {
  if (servers.length === 0) {
    return (
      <div>
        <Label>Servidores (0 seleccionados)</Label>
        <p className="text-[12px] text-[#8b95a5]">No hay servidores disponibles</p>
      </div>
    );
  }

  return (
    <div>
      <Label>Servidores ({selectedServers.length} seleccionados)</Label>
      <div className="space-y-2">
        <div className="flex gap-2 mb-2">
          <button onClick={onSelectAll} className="text-[11px] text-emerald-400 hover:text-emerald-300">
            Seleccionar todos
          </button>
          <span className="text-[#3a4558]">|</span>
          <button onClick={onDeselectAll} className="text-[11px] text-[#8b95a5] hover:text-white">
            Deseleccionar todos
          </button>
        </div>
        <div className="flex flex-wrap gap-2">
          {servers.map(server => (
            <button
              key={server}
              onClick={() => onToggle(server)}
              className={cn(
                "px-3 py-1.5 rounded-lg text-[12px] transition-colors",
                selectedServers.includes(server)
                  ? "bg-emerald-500/20 text-emerald-400 border border-emerald-500/30"
                  : "bg-[#0a0e17] text-[#8b95a5] border border-[#2a3548] hover:border-[#3a4558]"
              )}
            >
              {server}
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}


interface ConfigOptionsProps {
  config: ReportConfig;
  onUpdate: (updates: Partial<ReportConfig>) => void;
}

function ConfigOptions({ config, onUpdate }: ConfigOptionsProps) {
  const options = [
    { key: "includeAnomalies", label: "Anomalías detectadas" },
    { key: "includePredictions", label: "Predicciones generadas" },
    { key: "includeRecommendations", label: "Recomendaciones" },
  ] as const;

  return (
    <>
      <div className="grid grid-cols-2 gap-3">
        <div>
          <Label>Periodo</Label>
          <Select value={config.timeRange} onChange={e => onUpdate({ timeRange: e.target.value })}>
            <option value="1h">Ultima hora</option>
            <option value="6h">Ultimas 6 horas</option>
            <option value="24h">Ultimas 24 horas</option>
            <option value="7d">Ultimos 7 dias</option>
            <option value="30d">Ultimo mes</option>
          </Select>
        </div>
      </div>

      <div className="space-y-2">
        <Label>Contexto</Label>
        <div className="space-y-2">
          {options.map(opt => (
            <label key={opt.key} className="flex items-center gap-2 text-[13px] text-white cursor-pointer">
              <input
                type="checkbox"
                checked={config[opt.key]}
                onChange={e => onUpdate({ [opt.key]: e.target.checked })}
                className="rounded border-[#2a3548] bg-[#0a0e17] text-emerald-500 focus:ring-emerald-500"
              />
              {opt.label}
            </label>
          ))}
        </div>
      </div>
    </>
  );
}


interface HistoryToggleProps {
  count: number;
  isOpen: boolean;
  onToggle: () => void;
}

function HistoryToggle({ count, isOpen, onToggle }: HistoryToggleProps) {
  return (
    <button
      onClick={onToggle}
      className="w-full flex items-center justify-between px-3 py-2 text-[12px] text-[#8b95a5] hover:text-white border border-[#2a3548] rounded-lg transition-colors"
    >
      <span>Historial de Informes ({count})</span>
      {isOpen ? <ChevronUp className="w-4 h-4" /> : <ChevronDown className="w-4 h-4" />}
    </button>
  );
}


interface ReportHistoryProps {
  reports: GeneratedReport[];
  selectedReport: GeneratedReport | null;
  onSelect: (report: GeneratedReport) => void;
}

function ReportHistory({ reports, selectedReport, onSelect }: ReportHistoryProps) {
  return (
    <div className="space-y-2 max-h-[200px] overflow-y-auto">
      {reports.map(report => (
        <ReportHistoryItem
          key={report.id}
          report={report}
          isSelected={selectedReport?.id === report.id}
          onSelect={() => onSelect(report)}
        />
      ))}
    </div>
  );
}

interface ReportHistoryItemProps {
  report: GeneratedReport;
  isSelected: boolean;
  onSelect: () => void;
}

function ReportHistoryItem({ report, isSelected, onSelect }: ReportHistoryItemProps) {
  return (
    <div
      className={cn(
        "p-3 rounded-lg border transition-colors",
        isSelected
          ? "border-emerald-500 bg-emerald-500/10"
          : "border-[#2a3548] bg-[#0a0e17] hover:border-[#3a4558]"
      )}
    >
      <button onClick={onSelect} className="w-full text-left">
        <div className="flex items-center justify-between mb-1">
          <span className="text-[13px] font-medium text-white truncate">{report.title}</span>
          <Badge variant="success" size="xs">{report.type}</Badge>
        </div>
        <div className="flex items-center gap-3 text-[11px] text-[#8b95a5]">
          <span>{new Date(report.generatedAt).toLocaleDateString()}</span>
          <span>•</span>
          <span>{report.servers.length} servidor(es)</span>
        </div>
      </button>
      <div className="flex gap-2 mt-2 pt-2 border-t border-[#2a3548]">
        <button
          onClick={e => { e.stopPropagation(); exportDebugPrompt(report); }}
          className="flex items-center gap-1 px-2 py-1 text-[10px] text-amber-400 hover:text-amber-300 bg-amber-500/10 hover:bg-amber-500/20 rounded transition-colors"
        >
          <FileText className="w-3 h-3" />
          Debug
        </button>
        <button
          onClick={e => { e.stopPropagation(); exportToPDF(report); }}
          className="flex items-center gap-1 px-2 py-1 text-[10px] text-purple-400 hover:text-purple-300 bg-purple-500/10 hover:bg-purple-500/20 rounded transition-colors"
        >
          <Download className="w-3 h-3" />
          PDF
        </button>
        <button
          onClick={e => { e.stopPropagation(); exportToMarkdown(report); }}
          className="flex items-center gap-1 px-2 py-1 text-[10px] text-[#8b95a5] hover:text-white bg-[#1a2332] hover:bg-[#2a3548] rounded transition-colors"
        >
          <Download className="w-3 h-3" />
          MD
        </button>
      </div>
    </div>
  );
}


interface ReportPreviewProps {
  report: GeneratedReport;
}

function ReportPreview({ report }: ReportPreviewProps) {
  return (
    <div className="space-y-4">
      {/* Metrics Summary */}
      <MetricsSummary report={report} />

      {/* Report Content */}
      <Card variant="elevated" padding="md">
        <div className="flex items-center justify-between mb-3">
          <h4 className="text-[14px] font-medium text-white">{report.title}</h4>
          <div className="flex items-center gap-2">
            <button
              onClick={() => exportToPDF(report)}
              className="flex items-center gap-1 px-2 py-1 text-[11px] text-purple-400 hover:text-purple-300 transition-colors"
            >
              <Download className="w-3 h-3" />
              PDF
            </button>
          </div>
        </div>
        <div className="prose prose-invert prose-sm max-h-[400px] overflow-y-auto text-[12px] text-[#c5cdd8]">
          <ReactMarkdown remarkPlugins={[remarkGfm]}>{report.content}</ReactMarkdown>
        </div>
      </Card>
    </div>
  );
}

function MetricsSummary({ report }: { report: GeneratedReport }) {
  const metrics = [
    { key: "cpu", label: "CPU", color: "blue" },
    { key: "memory", label: "Memoria", color: "purple" },
    { key: "anomalies", label: "Anomalias", color: "yellow", value: report.anomaliesCount },
    { key: "predictions", label: "Predicciones", color: "cyan", value: report.predictionsCount },
  ];

  return (
    <div className="grid grid-cols-4 gap-2">
      {metrics.map(m => {
        const value = m.value !== undefined ? m.value : getMetricValue(report, m.key);
        const colorClasses = {
          blue: "bg-blue-500/10 border-blue-500/30 text-blue-400",
          purple: "bg-purple-500/10 border-purple-500/30 text-purple-400",
          yellow: "bg-yellow-500/10 border-yellow-500/30 text-yellow-400",
          cyan: "bg-cyan-500/10 border-cyan-500/30 text-cyan-400",
        };

        return (
          <div key={m.key} className={cn("p-3 rounded-lg border", colorClasses[m.color as keyof typeof colorClasses])}>
            <div className="text-[10px] mb-1">{m.label}</div>
            <div className="text-[16px] font-bold text-white">
              {m.key === "anomalies" || m.key === "predictions" ? value : `${value.toFixed(1)}%`}
            </div>
          </div>
        );
      })}
    </div>
  );
}
