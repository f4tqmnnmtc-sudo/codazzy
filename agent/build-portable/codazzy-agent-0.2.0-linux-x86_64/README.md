# Instalacion

1. Extraer el paquete:
   ```bash
   tar -xzf codazzy-agent-*.tar.gz
   cd codazzy-agent-*
   ```

2. Ejecutar el script de instalacion:
   ```bash
   ./install.sh
   ```

3. (Opcional) Editar la configuracion:
   ```bash
   nano /opt/codazzy-agent/config/config.toml
   ```

4. Iniciar el servicio:
   ```bash
   sudo systemctl start codazzy-agent
   ```

## Comandos

- Iniciar: `sudo systemctl start codazzy-agent`
- Detener: `sudo systemctl stop codazzy-agent`
- Estado: `sudo systemctl status codazzy-agent`
- Logs: `journalctl -u codazzy-agent -f`
- Reiniciar: `sudo systemctl restart codazzy-agent`

## Ficheros

- Binario: `/opt/codazzy-agent/codazzy-agent`
- Configuracion: `/opt/codazzy-agent/config/config.toml`
- Logs: `/opt/codazzy-agent/logs/`

## Desinstalar

```bash
./uninstall.sh
```
