#!/bin/bash
set -e

RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
BLUE='\033[0;34m'
NC='\033[0m'

print_banner() {
    echo -e "${BLUE}"
    echo "╔════════════════════════════════════════════════════════════════╗"
    echo "║                                                                ║"
    echo "║     ██████╗ ██████╗ ██████╗  █████╗ ███████╗███████╗██╗   ██╗  ║"
    echo "║    ██╔════╝██╔═══██╗██╔══██╗██╔══██╗╚══███╔╝╚══███╔╝╚██   ██╗  ║"
    echo "║    ██║     ██║   ██║██║  ██║███████║  ███╔╝   ███╔╝  ╚████╔╝   ║"
    echo "║    ██║     ██║   ██║██║  ██║██╔══██║ ███╔╝   ███╔╝   ██╔╝      ║"
    echo "║    ╚██████╗╚██████╔╝██████╔╝██║  ██║███████╗███████╗██╔╝       ║"
    echo "║     ╚═════╝ ╚═════╝ ╚═════╝ ╚═╝  ╚═╝╚══════╝╚══════╝╚═╝        ║"
    echo "║                                                                ║"
    echo "║         Sistema de Monitorizacion Predictiva                   ║"
    echo "╚════════════════════════════════════════════════════════════════╝"
    echo -e "${NC}"
}

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[OK]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

check_docker() {
    log_info "Verificando Docker..."
    if ! command -v docker &> /dev/null; then
        log_error "Docker no esta instalado"
        echo "Instala Docker: https://docs.docker.com/get-docker/"
        exit 1
    fi
    
    if ! docker info &> /dev/null; then
        log_error "Docker no esta corriendo o no tienes permisos"
        echo "Ejecuta: sudo systemctl start docker"
        exit 1
    fi
    log_success "Docker OK ($(docker --version | cut -d' ' -f3 | tr -d ','))"
}

check_docker_compose() {
    log_info "Verificando Docker Compose..."
    if docker compose version &> /dev/null; then
        COMPOSE_CMD="docker compose"
        log_success "Docker Compose OK (plugin)"
    elif command -v docker-compose &> /dev/null; then
        COMPOSE_CMD="docker-compose"
        log_success "Docker Compose OK (standalone)"
    else
        log_error "Docker Compose no esta instalado"
        exit 1
    fi
}

check_gpu() {
    log_info "Detectando GPU NVIDIA..."
    if command -v nvidia-smi &> /dev/null && nvidia-smi &> /dev/null; then
        GPU_AVAILABLE=true
        GPU_NAME=$(nvidia-smi --query-gpu=name --format=csv,noheader | head -1)
        log_success "GPU detectada: $GPU_NAME"
    else
        GPU_AVAILABLE=false
        log_warn "No se detecto GPU NVIDIA (se usara CPU)"
    fi
}

generate_secret() {
    openssl rand -hex 32 2>/dev/null || cat /dev/urandom | tr -dc 'a-zA-Z0-9' | fold -w 64 | head -n 1
}

create_env_file() {
    log_info "Configurando variables de entorno..."
    
    if [ -f .env ]; then
        log_warn "El archivo .env ya existe"
        read -p "Deseas sobrescribirlo? [y/N]: " overwrite
        if [[ ! "$overwrite" =~ ^[Yy]$ ]]; then
            log_info "Manteniendo .env existente"
            return
        fi
    fi
    
    POSTGRES_PWD=$(generate_secret | cut -c1-32)
    INFLUX_PWD=$(generate_secret | cut -c1-32)
    INFLUX_TKN=$(generate_secret)
    SECRET=$(generate_secret)
    
    cat > .env << EOF
# Codazzy - Generated $(date)

# Puertos
DASHBOARD_PORT=7245
GATEWAY_PORT=8000
PROFETA_PORT=9021
INFLUX_PORT=8086
POSTGRES_PORT=5432
NATS_PORT=4222
NATS_MONITOR_PORT=8222
REDIS_PORT=6379

# PostgreSQL
POSTGRES_DB=codazzy
POSTGRES_USER=codazzy
POSTGRES_PASSWORD=$POSTGRES_PWD

# InfluxDB
INFLUX_USER=admin
INFLUX_PASSWORD=$INFLUX_PWD
INFLUX_ORG=monitoring
INFLUX_BUCKET=metrics
INFLUX_TOKEN=$INFLUX_TKN

# Chronos ML
CHRONOS_MODEL=amazon/chronos-t5-base
USE_GPU=$USE_GPU

# Seguridad
SECRET_KEY=$SECRET
ALLOWED_ORIGINS=*

# OpenAI (opcional)
OPENAI_API_KEY=
OPENAI_MODEL=gpt-5-nano
EOF

    log_success "Archivo .env generado con credenciales seguras"
}

build_images() {
    log_info "Construyendo imagenes Docker (esto puede tardar varios minutos)..."
    
    $COMPOSE_CMD -f docker-compose.yml build --parallel
    
    log_success "Imagenes construidas correctamente"
}

start_services() {
    log_info "Iniciando servicios..."
    
    if [ "$USE_GPU" = "true" ]; then
        log_info "Iniciando con soporte GPU..."
        $COMPOSE_CMD -f docker-compose.yml --profile gpu up -d
    else
        log_info "Iniciando en modo CPU..."
        $COMPOSE_CMD -f docker-compose.yml up -d
    fi
}

wait_for_services() {
    log_info "Esperando a que los servicios esten listos..."
    
    base_services=("codazzy-influxdb" "codazzy-postgres" "codazzy-redis" "codazzy-nats")
    
    for service in "${base_services[@]}"; do
        echo -n "  $service: "
        for i in {1..30}; do
            status=$(docker inspect --format='{{.State.Health.Status}}' "$service" 2>/dev/null || echo "starting")
            if [ "$status" = "healthy" ]; then
                echo -e "${GREEN}OK${NC}"
                break
            elif [ "$status" = "unhealthy" ]; then
                echo -e "${YELLOW}running (unhealthy)${NC}"
                break
            elif [ $i -eq 30 ]; then
                # Verificar si al menos está corriendo
                if docker ps --format '{{.Names}}' | grep -q "^${service}$"; then
                    echo -e "${YELLOW}running${NC}"
                else
                    echo -e "${RED}not found${NC}"
                fi
            else
                echo -n "."
                sleep 1
            fi
        done
    done
    

    echo -n "  codazzy-gateway-rs: "
    for i in {1..120}; do
        if curl -s http://localhost:8000/health > /dev/null 2>&1; then
            echo -e "${GREEN}OK${NC}"
            break
        elif [ $i -eq 120 ]; then
            if docker ps --format '{{.Names}}' | grep -q "codazzy-gateway-rs"; then
                echo -e "${YELLOW}starting (compilando)${NC}"
            else
                echo -e "${RED}not found${NC}"
            fi
        else
            echo -n "."
            sleep 2
        fi
    done
    
    # Dashboard
    echo -n "  codazzy-dashboard: "
    for i in {1..60}; do
        if curl -s http://localhost:7245 > /dev/null 2>&1; then
            echo -e "${GREEN}OK${NC}"
            break
        elif [ $i -eq 60 ]; then
            if docker ps --format '{{.Names}}' | grep -q "codazzy-dashboard"; then
                echo -e "${YELLOW}starting${NC}"
            else
                echo -e "${RED}not found${NC}"
            fi
        else
            echo -n "."
            sleep 1
        fi
    done
    
    # Profeta (modelo ML tarda en cargar)
    echo -n "  codazzy-profeta (modelo ML): "
    for i in {1..90}; do
        if curl -s http://localhost:9021/health > /dev/null 2>&1; then
            echo -e "${GREEN}OK${NC}"
            break
        elif [ $i -eq 90 ]; then
            if docker ps --format '{{.Names}}' | grep -q "codazzy-profeta"; then
                echo -e "${YELLOW}cargando modelo${NC}"
            else
                echo -e "${RED}not found${NC}"
            fi
        else
            echo -n "."
            sleep 2
        fi
    done
}

print_success() {
    echo ""
    echo -e "${GREEN}╔═══════════════════════════════════════════════════════════╗${NC}"
    echo -e "${GREEN}║              INSTALACION COMPLETADA                       ║${NC}"
    echo -e "${GREEN}╚═══════════════════════════════════════════════════════════╝${NC}"
    echo ""
    echo -e "  ${BLUE}Dashboard:${NC}  http://localhost:${DASHBOARD_PORT:-7245}"
    echo -e "  ${BLUE}API:${NC}        http://localhost:${GATEWAY_PORT:-8000}"
    echo -e "  ${BLUE}Chronos:${NC}    http://localhost:${PROFETA_PORT:-9021}"
    echo -e "  ${BLUE}InfluxDB:${NC}   http://localhost:${INFLUX_PORT:-8086}"
    echo ""
    echo -e "  Comandos utiles:"
    echo -e "    ${YELLOW}make logs${NC}     - Ver logs"
    echo -e "    ${YELLOW}make stop${NC}     - Detener servicios"
    echo -e "    ${YELLOW}make start${NC}    - Iniciar servicios"
    echo -e "    ${YELLOW}make status${NC}   - Ver estado"
    echo ""
}

main() {
    print_banner
    
    cd "$(dirname "$0")"
    
    check_docker
    check_docker_compose
    check_gpu
    
    if [ "$GPU_AVAILABLE" = "true" ]; then
        read -p "Usar GPU NVIDIA para ML? [Y/n]: " use_gpu
        if [[ "$use_gpu" =~ ^[Nn]$ ]]; then
            USE_GPU=false
        else
            USE_GPU=true
        fi
    else
        USE_GPU=false
    fi
    
    create_env_file
    source .env
    
    build_images
    start_services
    wait_for_services
    print_success
}

main "$@"
