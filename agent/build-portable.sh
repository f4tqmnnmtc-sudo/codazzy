#!/bin/bash
set -e

VERSION="0.2.0"
BUILD_DIR="build-portable"
PKG="codazzy-agent-${VERSION}-linux-x86_64"

rm -rf "$BUILD_DIR"
mkdir -p "$BUILD_DIR/$PKG"

echo "Compilando..."
export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target x86_64-unknown-linux-gnu

cp "target/x86_64-unknown-linux-gnu/release/codazzy-agent" "$BUILD_DIR/$PKG/codazzy-agent"

cat > "$BUILD_DIR/$PKG/config.toml" << 'EOF'
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

[collection.processes]
enabled = true
interval_seconds = 300

[transport]
nats_url = "nats://${NATS_HOST:-nats}:4222"
compression = true

[metrics]
exclude_interfaces = ["lo", "docker0", "veth*"]
exclude_filesystems = ["tmpfs", "devtmpfs", "sysfs", "proc"]

[logging]
level = "info"
EOF

cat > "$BUILD_DIR/$PKG/install.sh" << 'EOF'
#!/bin/bash
set -e

DIR="/opt/codazzy-agent"
CFG="$DIR/config"

[ ! -f "./codazzy-agent" ] && { echo "Ejecuta desde el directorio del paquete"; exit 1; }

[[ $EUID -eq 0 ]] && SUDO="" || SUDO="sudo"

systemctl is-active --quiet codazzy-agent 2>/dev/null && $SUDO systemctl stop codazzy-agent

mkdir -p "$DIR" "$CFG"
cp "./codazzy-agent" "$DIR/"
chmod +x "$DIR/codazzy-agent"

NATS_HOST="${NATS_HOST:-nats}"
sed "s/\${NATS_HOST:-nats}/$NATS_HOST/g" "./config.toml" > "$CFG/config.toml"
$SUDO mkdir -p /etc/codazzy/agent
$SUDO cp "$CFG/config.toml" /etc/codazzy/agent/

$SUDO tee /etc/systemd/system/codazzy-agent.service >/dev/null << EOSVC
[Unit]
Description=Codazzy Agent
After=network.target

[Service]
Type=simple
ExecStart=$DIR/codazzy-agent --config /etc/codazzy/agent/config.toml
Restart=always
RestartSec=10

[Install]
WantedBy=multi-user.target
EOSVC

$SUDO systemctl daemon-reload
$SUDO systemctl enable codazzy-agent
$SUDO systemctl start codazzy-agent

echo "Instalado en $DIR"
echo "Config: $CFG/config.toml"
echo "Logs: journalctl -u codazzy-agent -f"
EOF

chmod +x "$BUILD_DIR/$PKG/install.sh"

cat > "$BUILD_DIR/$PKG/uninstall.sh" << 'EOF'
#!/bin/bash
set -e

[[ $EUID -eq 0 ]] && SUDO="" || SUDO="sudo"

$SUDO systemctl stop codazzy-agent 2>/dev/null || true
$SUDO systemctl disable codazzy-agent 2>/dev/null || true
$SUDO rm -f /etc/systemd/system/codazzy-agent.service
$SUDO systemctl daemon-reload

read -p "Eliminar /opt/codazzy-agent? [y/N]: " r
[[ "$r" =~ ^[Yy]$ ]] && rm -rf /opt/codazzy-agent

echo "Desinstalado"
EOF

chmod +x "$BUILD_DIR/$PKG/uninstall.sh"

echo "$VERSION" > "$BUILD_DIR/$PKG/VERSION"

cd "$BUILD_DIR"
tar -czf "${PKG}.tar.gz" "$PKG"

echo ""
echo "Creado: $BUILD_DIR/${PKG}.tar.gz"
echo "Tamaño: $(du -h "${PKG}.tar.gz" | cut -f1)"
