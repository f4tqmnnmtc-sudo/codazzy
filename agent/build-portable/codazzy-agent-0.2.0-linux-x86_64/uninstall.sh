#!/bin/bash
# Script de desinstalacion del agente Codazzy

set -e

INSTALL_DIR="/opt/codazzy-agent"
SERVICE_FILE="/etc/systemd/system/codazzy-agent.service"

if [[ $EUID -eq 0 ]]; then
    SUDO=""
else
    if command -v sudo >/dev/null 2>&1; then
        SUDO="sudo"
    else
        echo "Error: se requiere sudo"
        exit 1
    fi
fi

echo "Desinstalando agente Codazzy..."

if systemctl is-active --quiet codazzy-agent; then
    echo "Deteniendo servicio..."
    $SUDO systemctl stop codazzy-agent
fi

if systemctl is-enabled --quiet codazzy-agent; then
    echo "Deshabilitando servicio..."
    $SUDO systemctl disable codazzy-agent
fi

if [ -f "$SERVICE_FILE" ]; then
    echo "Eliminando fichero de servicio..."
    $SUDO rm "$SERVICE_FILE"
    $SUDO systemctl daemon-reload
fi

echo "Eliminar directorio de instalacion y logs? (s/N)"
read -r response
if [[ "$response" =~ ^[Ss]$ ]]; then
    if [ -d "$INSTALL_DIR" ]; then
        echo "Eliminando directorio de instalacion..."
        rm -rf "$INSTALL_DIR"
    fi
    echo "Eliminacion completa"
else
    echo "Se conservan los datos en $INSTALL_DIR"
fi

echo "Desinstalacion completada"
