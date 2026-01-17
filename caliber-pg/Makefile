# CALIBER PostgreSQL Extension Makefile
# For PGXN distribution and manual installation
#
# Requirements:
# - PostgreSQL 18+ (pg_config must be in PATH or PG_CONFIG set)
# - Rust toolchain with cargo
# - cargo-pgrx installed
#
# Usage:
#   make build       - Build the extension
#   make install     - Install to PostgreSQL
#   make test        - Run tests (requires pg_test feature fix)
#   make package     - Create PGXN distribution package
#   make clean       - Clean build artifacts

EXTENSION = caliber
EXTVERSION = 0.4.0

# Use pg_config from PATH or allow override
PG_CONFIG ?= pg_config

# Get PostgreSQL version
PGVER := $(shell $(PG_CONFIG) --version | sed 's/PostgreSQL //' | cut -d. -f1)

# Verify PostgreSQL 18+
ifeq ($(shell test $(PGVER) -lt 18 && echo yes),yes)
$(error CALIBER requires PostgreSQL 18 or later. Found PostgreSQL $(PGVER). Please upgrade or set PG_CONFIG to point to PostgreSQL 18+.)
endif

# Get installation directories
SHAREDIR := $(shell $(PG_CONFIG) --sharedir)
PKGLIBDIR := $(shell $(PG_CONFIG) --pkglibdir)
EXTDIR := $(SHAREDIR)/extension

# pgrx feature flag
PGFEATURE := pg$(PGVER)

.PHONY: all build install test package clean help

all: build

help:
	@echo "CALIBER PostgreSQL Extension"
	@echo ""
	@echo "Usage:"
	@echo "  make build     - Build the extension"
	@echo "  make install   - Install to PostgreSQL (may require sudo)"
	@echo "  make test      - Run tests"
	@echo "  make package   - Create PGXN distribution package"
	@echo "  make clean     - Clean build artifacts"
	@echo ""
	@echo "Environment:"
	@echo "  PG_CONFIG=$(PG_CONFIG)"
	@echo "  PostgreSQL version: $(PGVER)"
	@echo "  Feature flag: $(PGFEATURE)"

build:
	@echo "Building CALIBER extension for PostgreSQL $(PGVER)..."
	cargo pgrx package --pg-config $(PG_CONFIG) --features $(PGFEATURE)
	@echo "Build complete."

install: build
	@echo "Installing CALIBER extension..."
	install -d $(DESTDIR)$(EXTDIR)
	install -d $(DESTDIR)$(PKGLIBDIR)
	@# Find and copy extension files from pgrx output
	@PKGDIR=$$(find target -name "caliber_pg-$(PGFEATURE)" -type d | head -1); \
	if [ -z "$$PKGDIR" ]; then \
		echo "Error: Could not find package directory"; \
		exit 1; \
	fi; \
	echo "Installing from $$PKGDIR..."; \
	install -m 644 $$PKGDIR/usr/share/postgresql/$(PGVER)/extension/*.control $(DESTDIR)$(EXTDIR)/; \
	install -m 644 $$PKGDIR/usr/share/postgresql/$(PGVER)/extension/*.sql $(DESTDIR)$(EXTDIR)/; \
	install -m 755 $$PKGDIR/usr/lib/postgresql/$(PGVER)/lib/*.so $(DESTDIR)$(PKGLIBDIR)/
	@echo "Installation complete."
	@echo ""
	@echo "To enable the extension, run in psql:"
	@echo "  CREATE EXTENSION caliber;"
	@echo "  SELECT caliber_init();"

test:
	@echo "Running CALIBER tests..."
	@echo "Note: pg_test feature is currently broken with PG18 in pgrx."
	@echo "Running cargo tests instead..."
	cargo test --features $(PGFEATURE)

package:
	@echo "Creating PGXN distribution package..."
	@mkdir -p dist
	@VERSION=$(EXTVERSION); \
	PKGNAME=$(EXTENSION)-$$VERSION; \
	rm -rf dist/$$PKGNAME dist/$$PKGNAME.zip; \
	mkdir -p dist/$$PKGNAME; \
	cp META.json dist/$$PKGNAME/; \
	cp Makefile dist/$$PKGNAME/; \
	cp -r sql dist/$$PKGNAME/; \
	cp -r src dist/$$PKGNAME/; \
	cp Cargo.toml dist/$$PKGNAME/; \
	cd dist && zip -r $$PKGNAME.zip $$PKGNAME
	@echo "Package created: dist/$(EXTENSION)-$(EXTVERSION).zip"

clean:
	cargo clean
	rm -rf dist/

# Development targets
.PHONY: dev run-pg

dev:
	@echo "Starting development PostgreSQL instance..."
	cargo pgrx run $(PGFEATURE)

run-pg:
	@echo "Starting PostgreSQL with CALIBER extension..."
	cargo pgrx start $(PGFEATURE)
