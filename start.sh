#!/bin/bash
set -e

echo "Iniciando Codazzy..."

if ! command -v docker &> /dev/null; then
    echo "Error: Docker no instalado"
    exit 1
fi

echo "Levantando infraestructura..."
docker compose up -d influxdb nats redis postgres
sleep 10

echo "Levantando gateway..."
docker compose up -d gateway-rs
sleep 5

echo "Levantando dashboard y profeta..."
docker compose up -d dashboard profeta

echo ""
echo "Codazzy iniciado"
echo ""
echo "Dashboard:  http://localhost:7245"
echo "Gateway:    http://localhost:8000"
echo "Profeta:    http://localhost:9021"
echo "InfluxDB:   http://localhost:8086"
echo ""
echo "Logs: docker compose logs -f"
echo "Stop: docker compose down"
