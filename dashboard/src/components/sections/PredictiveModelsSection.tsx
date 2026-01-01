"use client";

import { useEffect } from "react";
import {
  LineChart,
  Line,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  Legend,
  ResponsiveContainer,
  Area,
  ComposedChart,
} from "recharts";
import { cn } from "@/lib/utils";
import { useForecast } from "@/hooks/useForecast";
import { Button, Select, Badge, Spinner, Card, ErrorBanner } from "@/components/ui/primitives";


export function PredictiveModelsSection() {
  const pm = useForecast();

  useEffect(() => {
    if (pm.selectedServer && pm.activeMetric) {
      pm.loadHistoricalData();
    }
  }, [pm.selectedServer, pm.activeMetric, pm.timeRange, pm.loadHistoricalData]);

  if (pm.loading && pm.servers.length === 0) {
    return (
      <div className="flex items-center justify-center p-8">
        <Spinner size="md" />
        <span className="ml-3 text-[#8b95a5]">Cargando servidores y métricas...</span>
      </div>
    );
  }

  return (
    <div className="space-y-4">
      {/* Chronos Status */}
      <ChronosStatus health={pm.chronosHealth} />

      {/* Server & Metric Selection */}
      <div className="grid grid-cols-2 gap-4">
        <ServerSelector
          servers={pm.servers}
          selectedServer={pm.selectedServer}
          onSelect={pm.selectServer}
          getServerInfo={pm.getServerInfo}
        />

        <MetricSelector
          metrics={pm.metrics}
          selectedMetrics={pm.selectedMetrics}
          serverInfo={pm.getServerInfo(pm.selectedServer)}
          onToggle={pm.toggleMetric}
        />
      </div>

      {/* Active Metric Tabs */}
      {pm.selectedMetrics.length > 0 && (
        <MetricTabs
          metrics={pm.selectedMetrics}
          activeMetric={pm.activeMetric}
          getMetricInfo={pm.getMetricInfo}
          onSelect={pm.setActiveMetric}
        />
      )}

      {/* Chart Section */}
      {pm.activeMetric && (
        <ChartSection
          pm={pm}
          onExportCSV={pm.exportToCSV}
          onExportJSON={pm.exportToJSON}
        />
      )}
    </div>
  );
}


function ChronosStatus({ health }: { health: { status: string; model_loaded: boolean } | null }) {
  return (
    <div className="flex items-center justify-between rounded-lg border border-[#2a3548] bg-[#0a0e17] p-3">
      <div className="flex items-center gap-2">
        <Badge variant={health?.model_loaded ? "success" : "warning"} size="sm">
          {health?.model_loaded ? "Chronos Disponible" : "Chronos No disponible"}
        </Badge>
      </div>
    </div>
  );
}


interface ServerSelectorProps {
  servers: ReturnType<typeof useForecast>["servers"];
  selectedServer: string;
  onSelect: (key: string) => void;
  getServerInfo: ReturnType<typeof useForecast>["getServerInfo"];
}

function ServerSelector({ servers, selectedServer, onSelect, getServerInfo }: ServerSelectorProps) {
  const serverInfo = getServerInfo(selectedServer);

  return (
    <div>
      <label className="block text-[12px] text-[#8b95a5] uppercase tracking-wide mb-1.5">Servidor</label>
      <Select value={selectedServer} onChange={e => onSelect(e.target.value)}>
        {servers.length === 0 ? (
          <option value="">No hay servidores</option>
        ) : (
          <>
            {servers.filter(s => s.server_type === "agent").length > 0 && (
              <optgroup label="Agentes Instalados">
                {servers
                  .filter(s => s.server_type === "agent")
                  .map(s => (
                    <option key={`${s.server_id}_agent`} value={`${s.server_id}_agent`}>
                      {s.server_name} {s.location && s.location !== "Unknown" ? `(${s.location})` : ""}
                    </option>
                  ))}
              </optgroup>
            )}
            {servers.filter(s => s.server_type === "remote").length > 0 && (
              <optgroup label="Conexiones Remotas">
                {servers
                  .filter(s => s.server_type === "remote")
                  .map(s => (
                    <option key={`${s.server_id}_remote`} value={`${s.server_id}_remote`}>
                      {s.server_name} [{s.protocol?.toUpperCase() || "SNMP"}]
                    </option>
                  ))}
              </optgroup>
            )}
          </>
        )}
      </Select>

      {serverInfo && (
        <div className="mt-2 rounded bg-[#131a26] p-2 text-[11px] text-[#8b95a5]">
          <div>{serverInfo.server_type === "agent" ? "🖥️ Agente Instalado" : "🌐 Conexión Remota"}</div>
          <div>
            Estado:{" "}
            <span className={serverInfo.status === "online" ? "text-emerald-400" : "text-red-400"}>
              {serverInfo.status}
            </span>
          </div>
        </div>
      )}
    </div>
  );
}


interface MetricSelectorProps {
  metrics: ReturnType<typeof useForecast>["metrics"];
  selectedMetrics: string[];
  serverInfo: ReturnType<typeof useForecast>["getServerInfo"] extends (k: string) => infer R ? R : never;
  onToggle: (type: string) => void;
}

function MetricSelector({ metrics, selectedMetrics, serverInfo, onToggle }: MetricSelectorProps) {
  const isAgent = serverInfo?.server_type === "agent";
  const agentMetricTypes = ["cpu", "memory", "disk", "network", "temperature"];

  const availableForServer = isAgent
    ? metrics.filter(m => agentMetricTypes.includes(m.metric_type))
    : metrics.filter(
        m =>
          !agentMetricTypes.includes(m.metric_type) &&
          (m.available_servers.includes(serverInfo?.server_id || "") ||
            serverInfo?.available_metrics?.includes(m.metric_type))
      );

  return (
    <div>
      <label className="block text-[12px] text-[#8b95a5] uppercase tracking-wide mb-1.5">Métricas (máx. 4)</label>
      <div className="max-h-[150px] overflow-y-auto rounded-lg border border-[#2a3548] bg-[#0a0e17] p-2">
        {availableForServer.length === 0 ? (
          <div className="p-2 text-[11px] text-amber-400">
            {isAgent ? "No hay metricas de agente disponibles" : "No hay metricas remotas disponibles"}
          </div>
        ) : (
          <div className="flex flex-wrap gap-1.5">
            {availableForServer.map(metric => (
              <button
                key={metric.metric_type}
                onClick={() => onToggle(metric.metric_type)}
                disabled={!selectedMetrics.includes(metric.metric_type) && selectedMetrics.length >= 4}
                className={cn(
                  "rounded border px-2 py-1 text-[11px] transition-all",
                  selectedMetrics.includes(metric.metric_type)
                    ? "border-emerald-600 bg-emerald-500 text-[#0a0e17]"
                    : "border-[#2a3548] bg-[#131a26] text-[#8b95a5] hover:border-[#3a4558]"
                )}
              >
                {metric.metric_name} {metric.unit && `(${metric.unit})`}
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}


interface MetricTabsProps {
  metrics: string[];
  activeMetric: string;
  getMetricInfo: ReturnType<typeof useForecast>["getMetricInfo"];
  onSelect: (type: string) => void;
}

function MetricTabs({ metrics, activeMetric, getMetricInfo, onSelect }: MetricTabsProps) {
  return (
    <div className="flex gap-1 rounded-lg border border-[#2a3548] bg-[#0a0e17] p-1">
      {metrics.map(metricType => {
        const metricInfo = getMetricInfo(metricType);
        return (
          <button
            key={metricType}
            onClick={() => onSelect(metricType)}
            className={cn(
              "flex-1 rounded px-3 py-2 text-[12px] transition-all",
              activeMetric === metricType
                ? "bg-emerald-500 font-medium text-[#0a0e17]"
                : "text-[#8b95a5] hover:bg-[#131a26]"
            )}
          >
            {metricInfo?.metric_name || metricType} {metricInfo?.unit && `(${metricInfo.unit})`}
          </button>
        );
      })}
    </div>
  );
}


interface ChartSectionProps {
  pm: ReturnType<typeof useForecast>;
  onExportCSV: () => void;
  onExportJSON: () => void;
}

function ChartSection({ pm, onExportCSV, onExportJSON }: ChartSectionProps) {
  const serverInfo = pm.getServerInfo(pm.selectedServer);
  const metricInfo = pm.getMetricInfo(pm.activeMetric);
  const canGenerate =
    !pm.generating && pm.chronosHealth?.model_loaded && pm.historicalData.length >= 10;

  return (
    <Card variant="elevated" padding="md">
      {/* Header */}
      <div className="mb-4 flex items-center justify-between">
        <div>
          <h4 className="text-[14px] font-medium text-white">
            {metricInfo?.metric_name || pm.activeMetric} - {serverInfo?.server_name}
          </h4>
          <p className="text-[11px] text-[#8b95a5]">Análisis predictivo basado en datos históricos</p>
        </div>
        {pm.forecastData && <Badge variant="success" size="sm">Predicción Activa</Badge>}
      </div>

      {/* Controls */}
      <div className="mb-4 flex flex-wrap items-center gap-3">
        <Select value={pm.timeRange} onChange={e => pm.setTimeRange(e.target.value)} size="sm" className="w-auto">
          <optgroup label="Corto plazo">
            <option value="24h">24 Horas</option>
            <option value="7d">7 Días</option>
          </optgroup>
          <optgroup label="Medio plazo">
            <option value="14d">2 Semanas</option>
            <option value="30d">1 Mes</option>
          </optgroup>
          <optgroup label="Largo plazo">
            <option value="90d">3 Meses</option>
            <option value="180d">6 Meses</option>
            <option value="365d">1 Año</option>
          </optgroup>
        </Select>

        <Select
          value={pm.aggregationMethod}
          onChange={e => pm.setAggregationMethod(e.target.value as "mean" | "median" | "max" | "min")}
          size="sm"
          className="w-auto"
        >
          <option value="mean">Promedio</option>
          <option value="median">Mediana</option>
          <option value="max">Máximo</option>
          <option value="min">Mínimo</option>
        </Select>

        <Select
          value={pm.forecastHorizon}
          onChange={e => pm.setForecastHorizon(e.target.value)}
          size="sm"
          className="w-auto"
        >
          <optgroup label="Horas">
            <option value="2 hours">2 Horas</option>
            <option value="4 hours">4 Horas</option>
            <option value="8 hours">8 Horas</option>
            <option value="12 hours">12 Horas</option>
          </optgroup>
          <optgroup label="Días">
            <option value="1 day">1 Día</option>
            <option value="3 days">3 Días</option>
            <option value="7 days">1 Semana</option>
            <option value="14 days">2 Semanas</option>
          </optgroup>
          <optgroup label="Largo plazo">
            <option value="30 days">1 Mes</option>
            <option value="90 days">3 Meses</option>
          </optgroup>
        </Select>

        <Button
          variant="primary"
          size="sm"
          onClick={pm.generateForecast}
          disabled={!canGenerate}
          loading={pm.generating}
        >
          {pm.generating ? "Generando..." : "Predecir"}
        </Button>

        {pm.historicalData.length > 0 && (
          <span className="text-[11px] text-[#8b95a5]">{pm.historicalData.length} puntos disponibles</span>
        )}
      </div>

      {/* Error */}
      {pm.error && <ErrorBanner message={pm.error} className="mb-4" />}

      {/* Rechart */}
      <ChartDisplay
        chartData={pm.chartData}
        forecastData={pm.forecastData}
        loading={pm.loading}
        historicalDataLength={pm.historicalData.length}
      />

      {/* Mostrar estadisticas forecast y la generación de CSV y JSON */}
      {/*pm.forecastData && (
        <ForecastStats
          forecastData={pm.forecastData}
          aggregationMethod={pm.aggregationMethod}
          metricUnit={metricInfo?.unit || "%"}
          onExportCSV={onExportCSV}
          onExportJSON={onExportJSON}
        />
      )*/}
    </Card>
  );
}


interface ChartDisplayProps {
  chartData: ReturnType<typeof useForecast>["chartData"];
  forecastData: ReturnType<typeof useForecast>["forecastData"];
  loading: boolean;
  historicalDataLength: number;
}

function ChartDisplay({ chartData, forecastData, loading, historicalDataLength }: ChartDisplayProps) {
  if (loading) {
    return (
      <div className="flex h-[300px] items-center justify-center">
        <Spinner size="md" />
        <span className="ml-3 text-[#8b95a5]">Cargando datos históricos...</span>
      </div>
    );
  }

  if (chartData.length === 0) {
    return (
      <div className="flex h-[300px] flex-col items-center justify-center text-center px-8">
        {historicalDataLength === 0 ? (
          <>
            <svg
              className="w-12 h-12 text-[#3a4558] mb-3"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M9 19v-6a2 2 0 00-2-2H5a2 2 0 00-2 2v6a2 2 0 002 2h2a2 2 0 002-2zm0 0V9a2 2 0 012-2h2a2 2 0 012 2v10m-6 0a2 2 0 002 2h2a2 2 0 002-2m0 0V5a2 2 0 012-2h2a2 2 0 012 2v14a2 2 0 01-2 2h-2a2 2 0 01-2-2z"
              />
            </svg>
            <p className="text-white font-medium">No hay datos históricos</p>
            <p className="text-[12px] text-[#8b95a5] mt-1">Selecciona un servidor y métrica para comenzar</p>
          </>
        ) : (
          <>
            <svg
              className="w-12 h-12 text-emerald-500/50 mb-3"
              fill="none"
              viewBox="0 0 24 24"
              stroke="currentColor"
            >
              <path
                strokeLinecap="round"
                strokeLinejoin="round"
                strokeWidth={1.5}
                d="M13 10V3L4 14h7v7l9-11h-7z"
              />
            </svg>
            <p className="text-white font-medium">Datos listos para predicción</p>
            <p className="text-[12px] text-[#8b95a5] mt-1">
              {historicalDataLength} puntos disponibles. Pulsa <span className="text-emerald-400">Predecir</span> para
              generar el pronóstico.
            </p>
          </>
        )}
      </div>
    );
  }

  return (
    <div className="h-[300px]">
      <ResponsiveContainer width="100%" height="100%">
        <ComposedChart data={chartData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
          <CartesianGrid strokeDasharray="3 3" stroke="#2a3548" />
          <XAxis
            dataKey="timeLabel"
            tick={{ fill: "#8b95a5", fontSize: 10 }}
            tickLine={{ stroke: "#2a3548" }}
            axisLine={{ stroke: "#2a3548" }}
            interval="preserveStartEnd"
          />
          <YAxis
            tick={{ fill: "#8b95a5", fontSize: 10 }}
            tickLine={{ stroke: "#2a3548" }}
            axisLine={{ stroke: "#2a3548" }}
            domain={["dataMin - 5", "dataMax + 5"]}
          />
          <Tooltip
            contentStyle={{
              backgroundColor: "#131a26",
              border: "1px solid #2a3548",
              borderRadius: "8px",
              fontSize: "11px",
            }}
            labelStyle={{ color: "#8b95a5" }}
          />
          <Legend wrapperStyle={{ fontSize: "11px" }} />

          {forecastData && (
            <>
              <Line
                type="monotone"
                dataKey="confidence_90"
                stroke="#f59e0b"
                strokeWidth={1}
                strokeDasharray="3 3"
                dot={false}
                name="Límite 90%"
                legendType="none"
              />
              <Line
                type="monotone"
                dataKey="confidence_10"
                stroke="#f59e0b"
                strokeWidth={1}
                strokeDasharray="3 3"
                dot={false}
                name="Límite 10%"
                legendType="none"
              />
              <Line
                type="monotone"
                dataKey="forecast_high"
                stroke="#22c55e"
                strokeWidth={1}
                strokeOpacity={0.6}
                dot={false}
                name="Límite 80%"
                legendType="none"
              />
              <Line
                type="monotone"
                dataKey="forecast_low"
                stroke="#22c55e"
                strokeWidth={1}
                strokeOpacity={0.6}
                dot={false}
                name="Límite 20%"
                legendType="none"
              />
              <Area
                type="monotone"
                dataKey="forecast_high"
                stroke="none"
                fill="#22c55e"
                fillOpacity={0.15}
                name="Intervalo confianza"
              />
            </>
          )}

          <Line
            type="monotone"
            dataKey="historical"
            stroke="#3b82f6"
            strokeWidth={2}
            dot={false}
            name="Datos Históricos"
            connectNulls={false}
          />
          {forecastData && (
            <Line
              type="monotone"
              dataKey="forecast_median"
              stroke="#2cd400"
              strokeWidth={2}
              strokeDasharray="5 5"
              dot={false}
              name="Predicción"
              connectNulls={false}
            />
          )}
        </ComposedChart>
      </ResponsiveContainer>
    </div>
  );
}


interface ForecastStatsProps {
  forecastData: NonNullable<ReturnType<typeof useForecast>["forecastData"]>;
  aggregationMethod: string;
  metricUnit: string;
  onExportCSV: () => void;
  onExportJSON: () => void;
}

function ForecastStats({ forecastData, aggregationMethod, metricUnit, onExportCSV, onExportJSON }: ForecastStatsProps) {
  return (
    <div className="mt-4 grid grid-cols-3 gap-4">
      {/* Processing Info */}
      <Card variant="default" padding="sm">
        <h5 className="mb-2 text-[11px] uppercase text-[#8b95a5]">Procesamiento</h5>
        <div className="space-y-1 text-[11px]">
          <StatRow label="Puntos enviados" value={forecastData.aggregation?.original_points || "-"} />
          <StatRow label="Puntos agregados" value={forecastData.aggregation?.aggregated_points || "-"} />
          <StatRow label="Método" value={aggregationMethod} capitalize />
          <StatRow label="Tiempo" value={`${forecastData.processing_time?.toFixed(2) || "-"}s`} />
          <StatRow label="Modelo" value={forecastData.model_info?.model_name || "chronos-t5-base"} small />
          <StatRow label="Hardware" value={forecastData.model_info?.device || "cuda"} />
        </div>
      </Card>

      {/* Analysis */}
      <Card variant="default" padding="sm">
        <h5 className="mb-2 text-[11px] uppercase text-[#8b95a5]">Análisis & Calidad</h5>
        {forecastData.analysis ? (
          <div className="space-y-1 text-[11px]">
            <StatRow
              label="Calidad"
              value={
                forecastData.analysis.confidence_score > 0.8
                  ? "Excelente"
                  : forecastData.analysis.confidence_score > 0.6
                    ? "Buena"
                    : "Regular"
              }
              valueColor="emerald"
            />
            <StatRow
              label="Confianza"
              value={`${Math.round((forecastData.analysis.confidence_score || forecastData.analysis.prediction_stability || 0.85) * 100)}%`}
            />
            <StatRow
              label="Tendencia"
              value={
                forecastData.analysis.trend === "increasing"
                  ? "Subiendo"
                  : forecastData.analysis.trend === "decreasing"
                    ? "Bajando"
                    : "Estable"
              }
              valueColor={
                forecastData.analysis.trend === "increasing"
                  ? "amber"
                  : forecastData.analysis.trend === "decreasing"
                    ? "emerald"
                    : undefined
              }
            />
            <StatRow label="Estabilidad" value={forecastData.analysis.stability || "Alta"} capitalize />
          </div>
        ) : (
          <div className="text-[11px] text-[#8b95a5]">Análisis no disponible</div>
        )}
      </Card>

      {/* Comparison & Export */}
      <div className="space-y-3">
        <Card variant="default" padding="sm">
          <h5 className="mb-2 text-[11px] uppercase text-[#8b95a5]">Comparación</h5>
          <div className="space-y-1 text-[11px]">
            <StatRow
              label="Media histórica"
              value={`${forecastData.analysis?.historical_stats?.mean?.toFixed(2) || "-"} ${metricUnit}`}
            />
            <StatRow
              label="Media predicción"
              value={`${forecastData.analysis?.prediction_stats?.mean?.toFixed(2) || "-"} ${metricUnit}`}
            />
          </div>
        </Card>

        <Card variant="default" padding="sm">
          <h5 className="mb-2 text-[11px] uppercase text-[#8b95a5]">Exportar</h5>
          <div className="flex gap-2">
            <Button variant="secondary" size="xs" onClick={onExportCSV} className="flex-1">
              CSV
            </Button>
            <Button variant="secondary" size="xs" onClick={onExportJSON} className="flex-1">
              JSON
            </Button>
          </div>
        </Card>
      </div>
    </div>
  );
}

function StatRow({
  label,
  value,
  capitalize,
  small,
  valueColor,
}: {
  label: string;
  value: string | number;
  capitalize?: boolean;
  small?: boolean;
  valueColor?: "emerald" | "amber" | "red";
}) {
  const colorClasses = {
    emerald: "text-emerald-400",
    amber: "text-amber-400",
    red: "text-red-400",
  };

  return (
    <div className="flex justify-between">
      <span className="text-[#8b95a5]">{label}:</span>
      <span
        className={cn(
          "text-white",
          capitalize && "capitalize",
          small && "text-[10px]",
          valueColor && colorClasses[valueColor]
        )}
      >
        {value}
      </span>
    </div>
  );
}
