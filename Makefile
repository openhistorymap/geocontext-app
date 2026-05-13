SHELL := /bin/bash
COMPOSE := docker compose
RUN := $(COMPOSE) run --rm app

.PHONY: help build shell install dev tauri-dev tauri-build check fmt clean nuke

help:
	@echo "make build         - build the Docker image"
	@echo "make install       - npm install inside container"
	@echo "make dev           - run SvelteKit dev server (Vite, no Tauri shell)"
	@echo "make tauri-dev     - run Tauri dev (needs X server / DISPLAY)"
	@echo "make tauri-build   - bundle release binaries"
	@echo "make check         - cargo check on the Rust side"
	@echo "make shell         - drop into a bash inside the container"
	@echo "make clean         - remove node_modules + cargo target volumes"
	@echo "make nuke          - clean + remove the image"

build:
	$(COMPOSE) build

icons:
	$(RUN) node scripts/gen-icon.cjs

install: icons
	$(RUN) npm install

dev:
	$(COMPOSE) run --rm --service-ports app npm run dev -- --host 0.0.0.0 --port 1420

tauri-dev:
	$(COMPOSE) run --rm --service-ports app npm run tauri -- dev

tauri-build:
	$(RUN) npm run tauri -- build

check:
	$(RUN) bash -lc "cd src-tauri && cargo check"

fmt:
	$(RUN) bash -lc "cd src-tauri && cargo fmt"

shell:
	$(RUN) bash

clean:
	docker volume rm -f geocontext-app_node_modules geocontext-app_cargo_target geocontext-app_cargo_registry geocontext-app_cargo_git || true

nuke: clean
	docker image rm -f geocontext-app:dev || true
