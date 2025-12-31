#!/bin/bash
# Script de instalacion del agente Codazzy (version portable)
# No requiere dependencias externas ni Rust

set -e

AGENT_NAME="codazzy-agent"
INSTALL_DIR="/opt/codazzy-agent"
CONFIG_DIR="$INSTALL_DIR/config"
LOG_DIR="$INSTALL_DIR/logs"
SERVICE_FILE="/etc/systemd/system/codazzy-agent.service"

echo "Instalando agente Codazzy..."

if [ ! -f "./codazzy-agent" ] || [ ! -f "./config.toml" ]; then
    echo "Error: ficheros de instalacion no encontrados"
    echo "Asegurate de ejecutar este script desde el directorio del paquete extraido"
    exit 1
fi

if [[ $EUID -eq 0 ]]; then
    SUDO=""
    echo "Ejecutando como root"
else
    if command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
        echo "Se usara sudo para operaciones privilegiadas"
    else
        echo "Error: se requieren permisos de root o sudo"
        exit 1
    fi
fi

if systemctl is-active --quiet codazzy-agent 2>/dev/null; then
    echo "Deteniendo servicio existente..."
    $SUDO systemctl stop codazzy-agent
    $SUDO systemctl disable codazzy-agent
    sleep 3
fi

echo "Creando directorios..."
mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$LOG_DIR"

echo "Instalando binario..."
if [ -f "$INSTALL_DIR/codazzy-agent" ]; then
    echo "Creando backup del binario anterior..."
    cp "$INSTALL_DIR/codazzy-agent" "$INSTALL_DIR/codazzy-agent.backup"
    rm -f "$INSTALL_DIR/codazzy-agent"
    sleep 2
fi

cp "./codazzy-agent" "$INSTALL_DIR/"
chmod +x "$INSTALL_DIR/codazzy-agent"

if [ ! -f "$INSTALL_DIR/codazzy-agent" ]; then
    echo "Error: fallo al copiar el binario"
    exit 1
fi

echo "Binario instalado"

if [ -z "$NATS_HOST" ]; then
    echo "NATS_HOST no definido, usando valor por defecto: nats"
    NATS_HOST="nats"
else
    echo "Usando NATS host: $NATS_HOST"
fi

echo "Instalando configuracion..."
cp "./config.toml" "$CONFIG_DIR/config.toml.tmp"
sed "s/\${NATS_HOST:-nats}/$NATS_HOST/g" "$CONFIG_DIR/config.toml.tmp" > "$CONFIG_DIR/config.toml"
rm -f "$CONFIG_DIR/config.toml.tmp"

$SUDO mkdir -p /etc/codazzy/agent
$SUDO cp "$CONFIG_DIR/config.toml" /etc/codazzy/agent/config.toml

echo "Creando servicio systemd..."
$SUDO tee "$SERVICE_FILE" > /dev/null << EOSERVICE
[Unit]
Description=Codazzy Monitoring Agent
After=network.target
Wants=network.target

[Service]
Type=simple
User=root
Group=root
WorkingDirectory=$INSTALL_DIR
ExecStart=$INSTALL_DIR/codazzy-agent --config /etc/codazzy/agent/config.toml
Restart=always
RestartSec=10
StandardOutput=journal
StandardError=journal

NoNewPrivileges=yes
ProtectSystem=strict
ProtectHome=no
ReadWritePaths=$INSTALL_DIR
ReadWritePaths=/etc/codazzy/agent
PrivateTmp=yes

[Install]
WantedBy=multi-user.target
EOSERVICE

echo "Recargando systemd..."
$SUDO systemctl daemon-reload

echo "Habilitando servicio..."
$SUDO systemctl enable codazzy-agent

echo "Iniciando servicio..."
$SUDO systemctl start codazzy-agent

sleep 2

if systemctl is-active --quiet codazzy-agent; then
    echo "Servicio iniciado correctamente"
else
    echo "El servicio esta iniciandose..."
fi

echo ""
echo "Instalacion completada"
echo "Directorio: $INSTALL_DIR"
echo "Configuracion: $CONFIG_DIR/config.toml"
echo "Logs: $LOG_DIR/"
echo ""
echo "Comandos utiles:"
echo "  - Estado: sudo systemctl status codazzy-agent"
echo "  - Logs: journalctl -u codazzy-agent -f"
echo "  - Reiniciar: sudo systemctl restart codazzy-agent"
echo "  - Detener: sudo systemctl stop codazzy-agent"
