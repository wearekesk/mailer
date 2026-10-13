# Mailer — local dev tasks.

.PHONY: help
help:
	@echo "make assets-install   install frontend dependencies via npm"
	@echo "make assets           bundle CSS and JS using Vite (npm run build)"
	@echo "make assets-watch     watch mode — rebuild assets on change"
	@echo "make run              cargo run"
	@echo "make dev              assets + cargo run"

.PHONY: assets-install
assets-install:
	npm install

.PHONY: assets
assets:
	npm run build

.PHONY: assets-watch
assets-watch:
	npm run dev

.PHONY: run
run:
	cargo run

.PHONY: dev
dev: assets
	cargo run
