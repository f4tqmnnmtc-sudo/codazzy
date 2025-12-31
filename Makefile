COMPOSE = docker compose

.PHONY: help install start stop restart logs logs-f status clean purge gpu build

help:
	@echo "Codazzy - Comandos disponibles:"
	@echo ""
	@echo "  install        Primera instalacion"
	@echo "  start          Iniciar servicios"
	@echo "  stop           Detener servicios"
	@echo "  restart        Reiniciar servicios"
	@echo "  status         Ver estado"
	@echo "  logs           Ver logs"
	@echo "  logs-f         Logs en tiempo real"
	@echo "  gpu            Iniciar con GPU"
	@echo "  build          Reconstruir imagenes"
	@echo "  clean          Eliminar contenedores"
	@echo "  purge          Eliminar todo (datos incluidos)"
	@echo ""

install:
	@./install.sh

start:
	@$(COMPOSE) up -d

stop:
	@$(COMPOSE) down

restart:
	@$(COMPOSE) restart

logs:
	@$(COMPOSE) logs --tail=100

logs-f:
	@$(COMPOSE) logs -f

status:
	@docker ps --filter "name=codazzy" --format "table {{.Names}}\t{{.Status}}"

clean:
	@$(COMPOSE) down --remove-orphans

purge:
	@echo "Esto eliminara todos los datos"
	@read -p "Continuar? [y/N]: " c && [ "$$c" = "y" ] || exit 1
	@$(COMPOSE) down -v --rmi local --remove-orphans

gpu:
	@$(COMPOSE) --profile gpu up -d

build:
	@$(COMPOSE) build --no-cache

logs-gateway:
	@docker logs -f codazzy-gateway-rs

logs-dashboard:
	@docker logs -f codazzy-dashboard

logs-profeta:
	@docker logs -f codazzy-profeta

shell-gateway:
	@docker exec -it codazzy-gateway-rs bash

shell-postgres:
	@docker exec -it codazzy-postgres psql -U codazzy

shell-redis:
	@docker exec -it codazzy-redis redis-cli
