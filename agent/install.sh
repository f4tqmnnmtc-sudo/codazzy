#!/bin/bash
set -e

AGENT_NAME="codazzy-agent"
SERVICE_USER="codazzy"
INSTALL_DIR="/opt/codazzy/agent"
CONFIG_DIR="/etc/codazzy/agent"
LOG_DIR="/var/log/codazzy"

OS=$(uname -s | tr '[:upper:]' '[:lower:]')
ARCH=$(uname -m)

case $ARCH in
    x86_64|amd64) ARCH="x86_64" ;;
    aarch64|arm64) ARCH="aarch64" ;;
    *) echo "Arquitectura no soportada: $ARCH"; exit 1 ;;
esac

[[ "$OS" != "linux" && "$OS" != "darwin" ]] && { echo "SO no soportado: $OS"; exit 1; }

echo "Sistema: $OS-$ARCH"

[[ $EUID -eq 0 ]] && SUDO="" || SUDO="sudo"

if [[ "$OS" == "linux" ]]; then
    if command -v apt-get >/dev/null; then
        $SUDO apt-get update -qq && $SUDO apt-get install -y curl wget >/dev/null
    elif command -v yum >/dev/null; then
        $SUDO yum install -y curl wget >/dev/null
    fi
fi

if [[ "$OS" == "linux" ]] && ! id "$SERVICE_USER" >/dev/null 2>&1; then
    $SUDO useradd --system --shell /bin/false --no-create-home "$SERVICE_USER"
fi

$SUDO mkdir -p "$INSTALL_DIR" "$CONFIG_DIR" "$LOG_DIR"
[[ "$OS" == "linux" ]] && $SUDO chown -R "$SERVICE_USER:$SERVICE_USER" "$INSTALL_DIR" "$LOG_DIR"

TEMP_DIR=$(mktemp -d)
cd "$TEMP_DIR"

if command -v cargo >/dev/null; then
    git clone --depth 1 https://github.com/codazzy/codazzy-agent.git
    cd codazzy-agent/agent
    cargo build --release
    $SUDO cp target/release/codazzy-agent "$INSTALL_DIR/$AGENT_NAME"
else
    echo "Rust no encontrado. Instala: curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh"
    exit 1
fi

$SUDO chmod +x "$INSTALL_DIR/$AGENT_NAME"
rm -rf "$TEMP_DIR"

CONFIG_FILE="$CONFIG_DIR/config.toml"
if [[ ! -f "$CONFIG_FILE" ]]; then
    $SUDO tee "$CONFIG_FILE" >/dev/null << 'EOF'
[node]
id = "auto"
environment = "production"

[collection]
interval_seconds = 5

[collection.hardware]
enabled = true

[collection.network]
enabled = true

[collection.storage]
enabled = true

[transport]
nats_url = "nats://localhost:4222"
compression = true

[metrics]
exclude_interfaces = ["lo", "docker0", "veth*"]
exclude_filesystems = ["tmpfs", "devtmpfs", "sysfs", "proc"]

[logging]
level = "info"
EOF
fi

if [[ "$OS" == "linux" ]]; then
    $SUDO tee /etc/systemd/system/codazzy-agent.service >/dev/null << EOF
[Unit]
Description=Codazzy Agent
After=network-online.target

[Service]
Type=simple
User=$SERVICE_USER
ExecStart=$INSTALL_DIR/$AGENT_NAME
WorkingDirectory=$CONFIG_DIR
Environment=CODAZZY_CONFIG_PATH=$CONFIG_DIR/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOF
    $SUDO systemctl daemon-reload
    $SUDO systemctl enable codazzy-agent
    $SUDO systemctl start codazzy-agent
fi

echo ""
echo "Instalado"
echo "  Binario: $INSTALL_DIR/$AGENT_NAME"
echo "  Config:  $CONFIG_DIR/config.toml"
echo ""
echo "Comandos:"
echo "  sudo systemctl status codazzy-agent"
echo "  sudo journalctl -u codazzy-agent -f"
