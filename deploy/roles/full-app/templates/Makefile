.PHONY: up down restart logs ps

COMPOSE_FILES := -f docker-compose.yml -f docker-compose.override.yml

up:
	docker compose $(COMPOSE_FILES) up -d --pull never

down:
	docker compose $(COMPOSE_FILES) down

restart:
	docker compose $(COMPOSE_FILES) restart

logs:
	docker compose $(COMPOSE_FILES) logs -f

ps:
	docker compose $(COMPOSE_FILES) ps
