# Agent

Agente de monitorización en Rust. Recolecta métricas y las envía al gateway vía NATS.

## Métricas

| Categoría | Datos |
|-----------|-------|
| Hardware | CPU por núcleo, memoria, sensores térmicos |
| Red | Interfaces, bytes TX/RX, errores |
| Almacenamiento | Discos I/O, filesystems |
| Procesos | Top CPU/memoria, servicios detectados |

## Instalación

```bash
cargo build --release
./install.sh
```

## Configuración

```toml
[node]
id = "servidor-01"
location = "datacenter-1"

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
nats_url = "nats://localhost:4222"
compression = true

[metrics]
exclude_interfaces = ["lo", "docker0"]
exclude_filesystems = ["tmpfs", "devtmpfs"]

[logging]
level = "info"
```

## Gestión

```bash
sudo systemctl status codazzy-agent
sudo systemctl start codazzy-agent
sudo systemctl stop codazzy-agent
sudo journalctl -u codazzy-agent -f
```

## Estructura

```
src/
├── main.rs
├── config.rs
├── error.rs
├── metrics.rs
├── transport.rs
└── collectors/
    ├── hardware.rs
    ├── network.rs
    ├── storage.rs
    └── processes.rs
```

## Dependencias

tokio, async-nats, sysinfo, serde, rmp-serde, lz4_flex, tracing

## Autor

Armando José Freitas Bontempo
