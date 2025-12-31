# Gateway

## Arquitectura

- Recibe métricas de agentes vía NATS JetStream (MessagePack + LZ4)
- Almacena en InfluxDB (time-series) y PostgreSQL (embeddings, alertas, documentos como contexto, información de dispositivos)
- Cache Redis para predicciones frecuentes
- WebSocket para streaming tiempo real
- Detección de anomalías por umbrales configurables
- Colección de dispositivos SNMP/SSH

## Endpoints en Uso (Dashboard)

### Health (para la instalación y conocer el estado de los contenedores)
```
GET /health
```

### Métricas
```
GET  /api/v1/metrics/timeseries    # Series temporales con filtros
GET  /api/v1/metrics/agents        # Lista de agentes activos
POST /api/v1/metrics/query         # Queries Flux personalizadas
POST /api/v1/metrics/cache/clear   # Limpiar cache
```

### Agentes (Instalación Remota)
```
GET    /api/v1/agents/remote-install           # Listar jobs
POST   /api/v1/agents/remote-install           # Crear job instalación
DELETE /api/v1/agents/remote-install           # Limpiar todos los jobs
GET    /api/v1/agents/remote-install/:job_id   # Estado de job
DELETE /api/v1/agents/remote-install/:job_id   # Cancelar job
GET    /api/v1/agents/installed-servers        # Servidores con agente
GET    /api/v1/agents/health-check/:hostname   # Test conexión SSH
POST   /api/v1/agents/uninstall                # Desinstalar agente
POST   /api/v1/agents/remote-config/fetch      # Obtener config remota
POST   /api/v1/agents/remote-config/update     # Actualizar config remota
```

### Alertas y Predicciones
```
GET  /api/v1/alerts/thresholds/:device_id   # Umbrales por dispositivo
POST /api/v1/alerts/thresholds/analyze      # Análisis IA de umbrales
GET  /api/v1/alerts/predictions             # Predicciones cacheadas
```

### Discovery (Red y Docker)
```
POST   /api/v1/discovery/scan/start         # Iniciar escaneo
GET    /api/v1/discovery/scan/:scan_id      # Estado del escaneo
GET    /api/v1/discovery/devices            # Dispositivos descubiertos
GET    /api/v1/discovery/devices/:device_id # Detalle dispositivo
PUT    /api/v1/discovery/devices/:device_id # Actualizar dispositivo
DELETE /api/v1/discovery/devices/:device_id # Eliminar dispositivo
GET    /api/v1/discovery/topology           # Topología de red
```

### Dispositivos Teleco (SNMP/SSH)
```
GET    /api/v1/teleco/devices              # Listar dispositivos
POST   /api/v1/teleco/devices              # Añadir dispositivo
DELETE /api/v1/teleco/devices/:device_id   # Eliminar dispositivo
```

### Documentos de Servidor
```
GET    /api/v1/servers/:node_id/documents         # Listar documentos
POST   /api/v1/servers/:node_id/documents/upload  # Subir documento
DELETE /api/v1/servers/:node_id/documents         # Eliminar todos
DELETE /api/v1/documents/:doc_id                  # Eliminar documento
```

### Predicciones (Chronos/Profeta)
```
POST /api/v1/predictions              # Guardar predicciones
GET  /api/v1/predictions/:node_id     # Predicciones por nodo
```

### Reportes IA
```
POST /api/reports/generate            # Generar reporte con OpenAI
POST /api/reports/export/:report_id   # Exportar a PDF/Markdown
```

### WebSocket
```
GET /ws        # Conexión WebSocket tiempo real
GET /ws/stats  # Estadísticas conexiones
```

## Ejecución

```bash
# Docker
docker compose up gateway-rs

# Local
cargo run --release
```

## Variables de Entorno

```env
# NATS
NATS_URL=nats://localhost:4222

# PostgreSQL
DATABASE_URL=postgresql://user:pass@localhost:5432/codazzy

# OpenAI (reportes con IA)
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-5-nano

# Predicciones
PREDICTION_ENABLED=true
PREDICTION_INTERVAL_SECS=300
```

## Autor

Armando José Freitas Bontempo