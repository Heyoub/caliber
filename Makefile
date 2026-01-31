# CALIBER Unified Development Makefile
# Run `make help` for available targets
#
# This is the SINGLE source of truth for all dev commands.
# Prefer `make <target>` over raw cargo/bun commands.
#
# Build cache location (if on slow drive, set in .cargo/config.toml):
#   [build]
#   target-dir = "/mnt/d/wsl-cache/target/caliber"

.PHONY: help test test-unit test-property test-fuzz test-chaos \
        test-integration test-e2e test-load test-security test-component \
        test-all test-llm llm-graphs journey-map llm-all bench lint ci ci-cloud clean dev setup \
        dev-api dev-frontend dev-all dev-watch \
        db-start db-stop db-status db-reset db-setup \
        pg-build pg-install pg-test pg-schema \
        check-deps

# Default target
.DEFAULT_GOAL := help

# Colors for terminal output
CYAN := \033[36m
GREEN := \033[32m
YELLOW := \033[33m
RESET := \033[0m

# Configuration
API_URL ?= http://localhost:3000
FUZZ_RUNS ?= 10000
CI_WORKFLOW ?= CI
CI_REF ?= main
CI_RUN_ID ?=

#==============================================================================
# Help
#==============================================================================

help: ## Show this help message
	@echo "$(CYAN)CALIBER Development$(RESET)"
	@echo ""
	@echo "$(GREEN)Usage:$(RESET) make [target]"
	@echo ""
	@echo "$(GREEN)🚀 Getting Started (first time):$(RESET)"
	@echo "  make setup         Install all dependencies"
	@echo "  make db-setup      Start DB + install extension"
	@echo "  make dev-api       Start API server (terminal 1)"
	@echo "  make dev-frontend  Start frontend (terminal 2)"
	@echo ""
	@echo "$(GREEN)🔧 Daily Development:$(RESET)"
	@echo "  make dev-api       Start API on :3000"
	@echo "  make dev-frontend  Start frontend on :5173"
	@echo "  make test          Run core tests"
	@echo "  make lint          Check code style"
	@echo "  make fmt           Auto-fix formatting"
	@echo ""
	@echo "$(GREEN)📦 After Rust Changes:$(RESET)"
	@echo "  make build         Build release binary"
	@echo "  make pg-install    Rebuild & install PG extension"
	@echo ""
	@echo "$(GREEN)Available Targets:$(RESET)"
	@grep -E '^[a-zA-Z_-]+:.*?## .*$$' $(MAKEFILE_LIST) | sort | awk 'BEGIN {FS = ":.*?## "}; {printf "  $(CYAN)%-18s$(RESET) %s\n", $$1, $$2}'

#==============================================================================
# Setup
#==============================================================================

setup: check-deps ## Install all development dependencies
	@echo "$(CYAN)Installing Rust tools...$(RESET)"
	cargo install cargo-nextest cargo-llvm-cov cargo-audit cargo-deny cargo-pgrx --version 0.16.1 --locked || true
	@echo "$(CYAN)Installing Node tools...$(RESET)"
	bun install
	cd app && bun install
	@echo "$(CYAN)Installing pre-commit hooks...$(RESET)"
	command -v lefthook >/dev/null 2>&1 && lefthook install || echo "Install lefthook: cargo install lefthook"
	@echo "$(GREEN)Setup complete!$(RESET)"
	@echo ""
	@echo "$(YELLOW)Next steps:$(RESET)"
	@echo "  1. make db-setup     # Start DB + install extension"
	@echo "  2. make dev-api      # Terminal 1: API server"
	@echo "  3. make dev-frontend # Terminal 2: Frontend"

check-deps: ## Check required dependencies are installed
	@echo "$(CYAN)Checking dependencies...$(RESET)"
	@command -v cargo >/dev/null 2>&1 || (echo "$(YELLOW)Missing: cargo (install Rust)$(RESET)" && exit 1)
	@command -v bun >/dev/null 2>&1 || (echo "$(YELLOW)Missing: bun (curl -fsSL https://bun.sh/install | bash)$(RESET)" && exit 1)
	@command -v psql >/dev/null 2>&1 || (echo "$(YELLOW)Missing: psql (install postgresql-client)$(RESET)" && exit 1)
	@echo "$(GREEN)All dependencies OK$(RESET)"

#==============================================================================
# Core Tests
#==============================================================================

test: test-unit test-property ## Run core tests (unit + property)

test-unit: ## Run unit tests only (Rust + TypeScript)
	@echo "$(CYAN)Running Rust unit tests...$(RESET)"
	$(MAKE) test-llm
	@echo "$(CYAN)Running TypeScript unit tests...$(RESET)"
	bun test ./tests/unit/ ./caliber-sdk/

test-rust: ## Run Rust tests only
	$(MAKE) test-llm

test-ts: ## Run TypeScript tests only
	bun test ./tests/ ./caliber-sdk/

#==============================================================================
# Advanced Tests
#==============================================================================

test-property: ## Run property-based tests (fast-check + proptest)
	@echo "$(CYAN)Running fast-check property tests...$(RESET)"
	bun test --test-name-pattern property
	@echo "$(CYAN)Running proptest property tests...$(RESET)"
	cargo test --workspace --exclude caliber-pg -- proptest || true

test-fuzz: ## Run fuzz tests
	@echo "$(CYAN)Running fuzz tests ($(FUZZ_RUNS) runs)...$(RESET)"
	FUZZ_RUNS=$(FUZZ_RUNS) bun test ./tests/fuzz/

test-chaos: ## Run chaos/resilience tests
	@echo "$(CYAN)Running chaos tests...$(RESET)"
	bun test ./tests/chaos/

test-integration: ## Run integration tests (requires DB)
	@echo "$(CYAN)Running integration tests...$(RESET)"
	cargo nextest run --workspace --exclude caliber-pg -E 'test(/db_backed/)' --test-threads=1

test-e2e: ## Run end-to-end tests (requires running API)
	@echo "$(CYAN)Running E2E tests against $(API_URL)...$(RESET)"
	CALIBER_API_URL=$(API_URL) bun test ./tests/e2e/

test-smoke: ## Run smoke tests (quick sanity check)
	@echo "$(CYAN)Running smoke tests...$(RESET)"
	bun test ./tests/smoke/

test-component: ## Run component tests
	@echo "$(CYAN)Running component tests...$(RESET)"
	bun test ./tests/component/

#==============================================================================
# LLM-Friendly Outputs
#==============================================================================

test-llm: ## Run Rust tests with LLM-friendly output + JUnit/JSON reports
	@echo "$(CYAN)Running Rust tests (LLM-friendly output)...$(RESET)"
	./scripts/llm/nextest_llm.sh

llm-graphs: ## Generate dependency graph + type signature index for LLMs
	@echo "$(CYAN)Generating dependency graph + type signature index...$(RESET)"
	node ./scripts/llm/gen_dep_graph.js
	node ./scripts/llm/gen_type_index.js

journey-map: ## Generate user journey map for LLMs
	@echo "$(CYAN)Generating user journey map...$(RESET)"
	node ./scripts/llm/gen_journey_map.js

llm-all: test-llm llm-graphs journey-map ## Run all LLM tooling

#==============================================================================
# Load & Performance Tests
#==============================================================================

test-load: test-load-baseline test-load-stress ## Run all load tests

test-load-baseline: ## Run baseline load test (establish p95 latency)
	@echo "$(CYAN)Running baseline load test...$(RESET)"
	k6 run tests/load/k6/api-baseline.js

test-load-stress: ## Run stress test (find breaking point)
	@echo "$(CYAN)Running stress test...$(RESET)"
	k6 run tests/load/k6/api-stress.js

#==============================================================================
# Security Tests
#==============================================================================

test-security: test-security-audit test-security-owasp ## Run all security tests

test-security-audit: ## Run cargo-audit and cargo-deny
	@echo "$(CYAN)Running cargo-audit...$(RESET)"
	cargo audit
	@echo "$(CYAN)Running cargo-deny...$(RESET)"
	cargo deny check

test-security-owasp: ## Run OWASP security tests
	@echo "$(CYAN)Running OWASP security tests...$(RESET)"
	CALIBER_API_URL=$(API_URL) bun test ./tests/security/

#==============================================================================
# Mutation Testing
#==============================================================================

test-mutation: ## Run mutation tests (Stryker)
	@echo "$(CYAN)Running mutation tests...$(RESET)"
	bunx stryker run

#==============================================================================
# Comprehensive
#==============================================================================

test-all: ## Run ALL tests (comprehensive, slow)
	@echo "$(CYAN)Running comprehensive test suite...$(RESET)"
	$(MAKE) lint
	$(MAKE) test-unit
	$(MAKE) test-property
	$(MAKE) test-fuzz FUZZ_RUNS=5000
	$(MAKE) test-chaos
	$(MAKE) test-smoke
	$(MAKE) test-component
	$(MAKE) test-integration
	$(MAKE) test-security-audit
	@echo "$(GREEN)All tests completed!$(RESET)"

#==============================================================================
# Benchmarks
#==============================================================================

bench: bench-rust bench-ts ## Run all benchmarks

bench-rust: ## Run Rust benchmarks (Criterion)
	@echo "$(CYAN)Running Rust benchmarks...$(RESET)"
	cargo bench --workspace --exclude caliber-pg

bench-ts: ## Run TypeScript benchmarks
	@echo "$(CYAN)Running TypeScript benchmarks...$(RESET)"
	bun test ./tests/bench/

#==============================================================================
# Linting & Formatting
#==============================================================================

lint: ## Run all linters
	@echo "$(CYAN)Checking Rust formatting...$(RESET)"
	cargo fmt --all -- --check
	@echo "$(CYAN)Running Clippy...$(RESET)"
	cargo clippy --workspace --all-targets --all-features -- -D warnings
	@echo "$(CYAN)Running Biome...$(RESET)"
	bunx biome check .

lint-fix: ## Fix linting issues automatically
	@echo "$(CYAN)Fixing Rust formatting...$(RESET)"
	cargo fmt --all
	@echo "$(CYAN)Fixing Biome issues...$(RESET)"
	bunx biome check --write .

fmt: lint-fix ## Alias for lint-fix

#==============================================================================
# CI Pipeline
#==============================================================================

ci: lint test-all build ## Full CI pipeline (lint + test + build)
	@echo "$(GREEN)CI pipeline completed successfully!$(RESET)"

ci-fast: ## Fast CI check (lint + unit tests only)
	@echo "$(CYAN)Running fast CI check...$(RESET)"
	$(MAKE) lint
	$(MAKE) test-unit

ci-cloud: ## Run CI in GitHub Actions and download logs
	@echo "$(CYAN)Running CI in GitHub Actions...$(RESET)"
	./scripts/ci/run-ci-and-fetch-logs.sh "$(CI_WORKFLOW)" "$(CI_REF)"

ci-logs: ## Download CI logs by run id (CI_RUN_ID=123)
	@if [ -z "$(CI_RUN_ID)" ]; then \
		echo "Usage: make ci-logs CI_RUN_ID=<run_id>"; \
		exit 1; \
	fi
	./scripts/ci/fetch-gh-logs.sh "$(CI_RUN_ID)"

#==============================================================================
# Build
#==============================================================================

build: ## Build release binary
	@echo "$(CYAN)Building release binary...$(RESET)"
	cargo build --release -p caliber-api

build-dev: ## Build debug binary
	cargo build -p caliber-api

#==============================================================================
# Development (PRIMARY ENTRY POINTS)
#==============================================================================

dev: dev-api ## Alias for dev-api

dev-api: ## Start API server (port 3000)
	@echo "$(CYAN)Starting API server on http://localhost:3000...$(RESET)"
	cargo run -p caliber-api --bin caliber-api

dev-frontend: ## Start frontend dev server (port 5173)
	@echo "$(CYAN)Starting frontend on http://localhost:5173...$(RESET)"
	cd app && bun run dev

dev-all: ## Start both API and frontend (requires 2 terminals)
	@echo "$(YELLOW)Run in separate terminals:$(RESET)"
	@echo "  Terminal 1: make dev-api"
	@echo "  Terminal 2: make dev-frontend"
	@echo ""
	@echo "Or use: make dev-api & make dev-frontend"

dev-watch: ## Start API with auto-reload on changes
	cargo watch -x 'run -p caliber-api --bin caliber-api'

#==============================================================================
# Database & PostgreSQL Extension
#==============================================================================

db-start: ## Start PostgreSQL (WSL)
	sudo service postgresql start

db-stop: ## Stop PostgreSQL (WSL)
	sudo service postgresql stop

db-status: ## Check PostgreSQL status
	@pg_isready -h localhost -p 5432 && echo "$(GREEN)PostgreSQL is running$(RESET)" || echo "$(YELLOW)PostgreSQL is not running$(RESET)"

db-reset: ## Reset test database
	@echo "$(CYAN)Resetting test database...$(RESET)"
	psql -U caliber -d postgres -c "DROP DATABASE IF EXISTS caliber_test;"
	psql -U caliber -d postgres -c "CREATE DATABASE caliber_test;"
	psql -U caliber -d caliber_test -c "CREATE EXTENSION IF NOT EXISTS vector;"

db-setup: db-start pg-install ## Full database setup (start + install extension)
	@echo "$(GREEN)Database ready!$(RESET)"

pg-build: ## Build caliber-pg extension
	@echo "$(CYAN)Building caliber-pg extension...$(RESET)"
	cd caliber-pg && cargo build --features pg18 --release

pg-install: ## Install caliber-pg extension to PostgreSQL 18
	@echo "$(CYAN)Installing caliber-pg extension...$(RESET)"
	cd caliber-pg && cargo pgrx install --pg-config=/usr/lib/postgresql/18/bin/pg_config --features pg18 --release
	@echo "$(CYAN)Recreating extension in database...$(RESET)"
	sudo -u postgres psql -d caliber -c "DROP EXTENSION IF EXISTS caliber_pg CASCADE; CREATE EXTENSION caliber_pg;"
	@echo "$(GREEN)Extension installed!$(RESET)"

pg-test: ## Run caliber-pg tests
	@echo "$(CYAN)Running caliber-pg tests...$(RESET)"
	cd caliber-pg && cargo pgrx test pg18 --features pg_test

pg-schema: ## Generate SQL schema from extension
	@echo "$(CYAN)Generating extension schema...$(RESET)"
	cd caliber-pg && cargo pgrx schema --features pg18

#==============================================================================
# Cleanup
#==============================================================================

clean: ## Clean build artifacts
	@echo "$(CYAN)Cleaning build artifacts...$(RESET)"
	cargo clean
	rm -rf node_modules/.cache
	rm -rf coverage/
	rm -rf reports/

clean-all: clean ## Clean everything including dependencies
	rm -rf node_modules/
	rm -rf target/

#==============================================================================
# Coverage
#==============================================================================

coverage: ## Generate test coverage report
	@echo "$(CYAN)Generating coverage report...$(RESET)"
	cargo llvm-cov --workspace --exclude caliber-pg --html --output-dir coverage/rust
	bun test --coverage
	@echo "$(GREEN)Coverage reports generated in coverage/$(RESET)"

coverage-open: coverage ## Generate and open coverage report
	open coverage/rust/html/index.html || xdg-open coverage/rust/html/index.html
