# Codazzy - Guía de Instalación

Sistema de Monitorización Predictiva con IA para infraestructura IT.

## Requisitos

### Mínimos
- Docker 24.0+
- Docker Compose v2.20+
- 8GB RAM
- 20GB disco libre

### Recomendados
- 16GB+ RAM
- GPU NVIDIA con 6GB+ VRAM (para predicciones ML)
- SSD

## Instalación

```bash
git clone https://github.com/tu-repo/codazzy.git
cd codazzy
./install.sh
```

El instalador detecta GPU, genera credenciales y levanta todos los servicios.

## Servicios

| Servicio | Puerto | Descripción |
|----------|--------|-------------|
| Dashboard | 7245 | Interfaz web |
| Gateway | 8000 | API REST (Rust) |
| Profeta | 9021 | Predicciones ML (Chronos) |
| InfluxDB | 8086 | Métricas temporales |
| PostgreSQL | 5432 | Datos relacionales |
| NATS | 4222 | Mensajería |
| Redis | 6379 | Caché |

## URLs

- **Dashboard:** http://localhost:7245
- **API:** http://localhost:8000
- **API Health:** http://localhost:8000/health
- **Profeta:** http://localhost:9021
- **InfluxDB:** http://localhost:8086

## Comandos

```bash
# Gestión básica
docker compose up -d          # Iniciar
docker compose down           # Detener
docker compose logs -f        # Ver logs

# Con GPU
docker compose --profile gpu up -d

# Reconstruir
docker compose build --no-cache
```

## Configuración

Edita `.env`:

```env
# Puertos
DASHBOARD_PORT=7245
GATEWAY_PORT=8000
PROFETA_PORT=9021

# PostgreSQL
POSTGRES_PASSWORD=tu_password

# InfluxDB
INFLUX_TOKEN=tu_token

# OpenAI (opcional, para reportes IA)
OPENAI_API_KEY=sk-...
OPENAI_MODEL=gpt-5-nano
```

## Modelos Chronos

| Modelo | Parámetros | RAM | Velocidad |
|--------|------------|-----|-----------|
| chronos-t5-tiny | 8M | 2GB | Muy rápida |
| chronos-t5-mini | 20M | 3GB | Rápida |
| chronos-t5-small | 46M | 4GB | Media |
| chronos-t5-base | 200M | 6GB | Lenta |
| chronos-t5-large | 710M | 12GB | Muy lenta |

Por defecto usa `chronos-t5-base`. Cambia en `.env`:
```env
CHRONOS_MODEL=amazon/chronos-t5-small
```

## GPU NVIDIA

Requisitos:
- Driver NVIDIA 525+
- NVIDIA Container Toolkit

```bash
# Verificar
nvidia-smi
docker run --rm --gpus all nvidia/cuda:12.0-base nvidia-smi

# Activar en .env
USE_GPU=true
```

## Agente de Monitorización

El agente se compila en `agent/` y se distribuye a servidores remotos vía SSH desde el dashboard.

```bash
cd agent
cargo build --release
```

El binario queda en `agent/target/release/codazzy-agent`.

## Troubleshooting

### Gateway no responde
```bash
docker logs codazzy-gateway-rs
# Si está compilando, esperar ~2 min
```

### Profeta tarda en iniciar
Normal. El modelo ML tarda 1-3 min en cargar.

### Sin espacio
```bash
docker system prune -a
```

### Ver estado
```bash
docker ps --format "table {{.Names}}\t{{.Status}}"
```

## Actualización

```bash
git pull
docker compose build
docker compose up -d
```

## Desinstalación

```bash
docker compose down -v  # Elimina contenedores y volúmenes
```
