#!/bin/bash
# CALIBER SDK Generation Script
#
# Generates client SDKs from OpenAPI spec and Protocol Buffers.
# Supports: TypeScript, Python, Go, Elixir
#
# Prerequisites:
#   - Rust toolchain (cargo)
#   - openapi-generator-cli (npm install -g @openapitools/openapi-generator-cli)
#   - protoc (for gRPC SDKs)
#   - For Go: go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
#            go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"
SDK_DIR="$ROOT_DIR/sdks"
SPEC_FILE="$ROOT_DIR/openapi.json"
PROTO_DIR="$ROOT_DIR/caliber-api/proto"
SDK_VERSION="${SDK_VERSION:-0.1.0}"

# Colors for output
RED='\033[0;31m'
GREEN='\033[0;32m'
BLUE='\033[0;34m'
YELLOW='\033[1;33m'
NC='\033[0m' # No Color

log_info() { echo -e "${BLUE}[INFO]${NC} $1"; }
log_success() { echo -e "${GREEN}[SUCCESS]${NC} $1"; }
log_warn() { echo -e "${YELLOW}[WARN]${NC} $1"; }
log_error() { echo -e "${RED}[ERROR]${NC} $1"; }

# Generate OpenAPI spec from Rust
generate_openapi_spec() {
    log_info "Generating OpenAPI specification..."

    if ! cargo run -p caliber-api --bin generate-openapi --features openapi > "$SPEC_FILE" 2>/dev/null; then
        log_error "Failed to generate OpenAPI spec. Ensure caliber-api builds with openapi feature."
        exit 1
    fi

    log_success "OpenAPI spec written to $SPEC_FILE"
}

# Validate OpenAPI spec
validate_spec() {
    log_info "Validating OpenAPI specification..."

    if command -v openapi-generator-cli &> /dev/null; then
        if openapi-generator-cli validate -i "$SPEC_FILE" 2>/dev/null; then
            log_success "OpenAPI spec is valid"
        else
            log_warn "OpenAPI spec has validation warnings (may still work)"
        fi
    else
        log_warn "openapi-generator-cli not found, skipping validation"
    fi
}

# Generate TypeScript SDK
generate_typescript() {
    log_info "Generating TypeScript SDK..."

    local sdk_dir="$ROOT_DIR/caliber-sdk"
    local gen_dir="$sdk_dir/src/generated"

    # Clean generated directory but preserve src/ if it exists
    rm -rf "$gen_dir"
    mkdir -p "$sdk_dir"

    openapi-generator-cli generate \
        -i "$SPEC_FILE" \
        -g typescript-axios \
        -o "$gen_dir" \
        --additional-properties=npmName=@caliber-run/sdk \
        --additional-properties=npmVersion="$SDK_VERSION" \
        --additional-properties=supportsES6=true \
        --additional-properties=withSeparateModelsAndApi=true \
        --additional-properties=modelPackage=models \
        --additional-properties=apiPackage=api \
        --additional-properties=withInterfaces=true \
        --global-property=skipFormModel=true

    # Create package.json if it doesn't exist (preserve custom changes)
    if [[ ! -f "$sdk_dir/package.json" ]]; then
        cat > "$sdk_dir/package.json" << EOF
{
  "name": "@caliber-run/sdk",
  "version": "${SDK_VERSION}",
  "description": "TypeScript SDK for CALIBER memory framework",
  "author": "Caliber",
  "license": "AGPL-3.0-or-later",
  "repository": {
    "type": "git",
    "url": "https://github.com/caliber-run/caliber.git",
    "directory": "caliber-sdk"
  },
  "homepage": "https://caliber.run",
  "main": "./dist/index.js",
  "module": "./dist/index.mjs",
  "types": "./dist/index.d.ts",
  "exports": {
    ".": {
      "import": {
        "types": "./dist/index.d.mts",
        "default": "./dist/index.mjs"
      },
      "require": {
        "types": "./dist/index.d.ts",
        "default": "./dist/index.js"
      }
    },
    "./generated": {
      "import": "./dist/generated/index.js",
      "require": "./dist/generated/index.js",
      "types": "./dist/generated/index.d.ts"
    }
  },
  "files": [
    "dist",
    "README.md"
  ],
  "scripts": {
    "build": "tsup",
    "dev": "tsup --watch",
    "test": "vitest",
    "test:run": "vitest run",
    "typecheck": "tsc --noEmit",
    "prepublishOnly": "npm run build",
    "clean": "rm -rf dist"
  },
  "dependencies": {
    "axios": "^1.6.0"
  },
  "devDependencies": {
    "@types/node": "^20.11.0",
    "tsup": "^8.0.1",
    "typescript": "^5.3.3",
    "vitest": "^1.2.0"
  },
  "engines": {
    "node": ">=18.0.0"
  },
  "keywords": [
    "caliber",
    "memory",
    "ai",
    "agents",
    "llm",
    "context",
    "sdk",
    "typescript"
  ],
  "publishConfig": {
    "access": "public"
  }
}
EOF
    fi

    # Create tsconfig.json
    cat > "$sdk_dir/tsconfig.json" << 'EOF'
{
  "compilerOptions": {
    "target": "ES2022",
    "module": "ESNext",
    "moduleResolution": "bundler",
    "lib": ["ES2022"],
    "declaration": true,
    "declarationMap": true,
    "sourceMap": true,
    "outDir": "./dist",
    "rootDir": "./",
    "strict": true,
    "esModuleInterop": true,
    "skipLibCheck": true,
    "forceConsistentCasingInFileNames": true,
    "resolveJsonModule": true,
    "isolatedModules": true
  },
  "include": ["src/**/*.ts"],
  "exclude": ["node_modules", "dist"]
}
EOF

    # Create tsup.config.ts
    cat > "$sdk_dir/tsup.config.ts" << 'EOF'
import { defineConfig } from 'tsup';

export default defineConfig({
  entry: ['src/index.ts'],
  format: ['cjs', 'esm'],
  dts: true,
  splitting: false,
  sourcemap: true,
  clean: true,
  treeshake: true,
  minify: false,
});
EOF

    log_success "TypeScript SDK generated at $sdk_dir"
    log_info "Generated API code is in $gen_dir"
    log_info "Ergonomic wrapper should be in $sdk_dir/src"
}

# Generate Python SDK
generate_python() {
    log_info "Generating Python SDK..."

    local out_dir="$SDK_DIR/python"
    rm -rf "$out_dir"

    openapi-generator-cli generate \
        -i "$SPEC_FILE" \
        -g python \
        -o "$out_dir" \
        --additional-properties=packageName=caliber_sdk \
        --additional-properties=projectName=caliber-sdk \
        --additional-properties=packageVersion="$SDK_VERSION" \
        --additional-properties=generateSourceCodeOnly=false \
        --global-property=skipFormModel=true

    # Add py.typed marker for PEP 561
    touch "$out_dir/caliber_sdk/py.typed"

    log_success "Python SDK generated at $out_dir"
}

# Generate Go SDK
generate_go() {
    log_info "Generating Go SDK..."

    local out_dir="$SDK_DIR/go"
    rm -rf "$out_dir"
    mkdir -p "$out_dir"

    # Generate REST client from OpenAPI
    openapi-generator-cli generate \
        -i "$SPEC_FILE" \
        -g go \
        -o "$out_dir" \
        --additional-properties=packageName=caliber \
        --additional-properties=packageVersion="$SDK_VERSION" \
        --additional-properties=isGoSubmodule=true \
        --additional-properties=generateInterfaces=true \
        --global-property=skipFormModel=true

    # Generate gRPC client from proto if proto exists
    if [[ -f "$PROTO_DIR/caliber.proto" ]]; then
        log_info "Generating Go gRPC client from proto..."

        mkdir -p "$out_dir/grpc"
        protoc \
            --go_out="$out_dir/grpc" \
            --go_opt=paths=source_relative \
            --go-grpc_out="$out_dir/grpc" \
            --go-grpc_opt=paths=source_relative \
            -I "$PROTO_DIR" \
            "$PROTO_DIR/caliber.proto" 2>/dev/null || log_warn "gRPC generation skipped (protoc not configured)"
    fi

    log_success "Go SDK generated at $out_dir"
}

# Generate Elixir SDK
generate_elixir() {
    log_info "Generating Elixir SDK..."

    local out_dir="$SDK_DIR/elixir"
    rm -rf "$out_dir"

    openapi-generator-cli generate \
        -i "$SPEC_FILE" \
        -g elixir \
        -o "$out_dir" \
        --additional-properties=packageName=caliber \
        --additional-properties=invokerPackage=Caliber \
        --global-property=skipFormModel=true

    # Create proper mix.exs with dependencies
    cat > "$out_dir/mix.exs" << EOF
defmodule Caliber.MixProject do
  use Mix.Project

  @version "${SDK_VERSION}"
  @source_url "https://github.com/caliber-run/caliber"

  def project do
    [
      app: :caliber,
      version: @version,
      elixir: "~> 1.14",
      start_permanent: Mix.env() == :prod,
      deps: deps(),
      package: package(),
      docs: docs(),
      name: "Caliber",
      description: "Elixir SDK for CALIBER - Cognitive Agent Long-term Intelligence, Behavioral Episodic Recall",
      source_url: @source_url
    ]
  end

  def application do
    [
      extra_applications: [:logger, :ssl, :inets]
    ]
  end

  defp deps do
    [
      {:tesla, "~> 1.7"},
      {:hackney, "~> 1.18"},
      {:jason, "~> 1.4"},
      {:ex_doc, "~> 0.31", only: :dev, runtime: false},
      {:dialyxir, "~> 1.4", only: [:dev, :test], runtime: false}
    ]
  end

  defp package do
    [
      maintainers: ["CALIBER Team"],
      licenses: ["AGPL-3.0-or-later"],
      links: %{
        "GitHub" => @source_url,
        "Docs" => "https://docs.caliber.run"
      }
    ]
  end

  defp docs do
    [
      main: "readme",
      extras: ["README.md"],
      source_ref: "v#{@version}"
    ]
  end
end
EOF

    # Create a README for the Elixir SDK
    cat > "$out_dir/README.md" << 'EOF'
# Caliber Elixir SDK

Elixir client for the CALIBER API - Cognitive Agent Long-term Intelligence, Behavioral Episodic Recall.

## Installation

Add `caliber` to your list of dependencies in `mix.exs`:

```elixir
def deps do
  [
    {:caliber, "~> 0.1.0"}
  ]
end
```

## Configuration

```elixir
# config/config.exs
config :caliber,
  base_url: "https://api.caliber.run",
  api_key: System.get_env("CALIBER_API_KEY")
```

## Usage

```elixir
# Create a trajectory
{:ok, trajectory} = Caliber.Trajectories.create(%{
  goal: "Build an AI agent",
  tenant_id: "your-tenant-id"
})

# List artifacts
{:ok, artifacts} = Caliber.Artifacts.list(trajectory_id: trajectory.id)

# Create a note (long-term memory)
{:ok, note} = Caliber.Notes.create(%{
  content: "Important discovery about user preferences",
  note_type: "insight",
  tenant_id: "your-tenant-id"
})
```

## License

AGPL-3.0-or-later - see [LICENSE](LICENSE) for details.
EOF

    log_success "Elixir SDK generated at $out_dir"
}

# Generate all SDKs
generate_all() {
    mkdir -p "$SDK_DIR"

    generate_openapi_spec
    validate_spec

    if command -v openapi-generator-cli &> /dev/null; then
        generate_typescript
        generate_python
        generate_go
        generate_elixir
    else
        log_error "openapi-generator-cli not found!"
        log_info "Install with: npm install -g @openapitools/openapi-generator-cli"
        exit 1
    fi

    echo ""
    log_success "All SDKs generated successfully!"
    echo ""
    echo "Generated SDKs:"
    echo "  - TypeScript: $ROOT_DIR/caliber-sdk (@caliber-run/sdk)"
    echo "  - Python:     $SDK_DIR/python"
    echo "  - Go:         $SDK_DIR/go"
    echo "  - Elixir:     $SDK_DIR/elixir"
    echo ""
    echo "Next steps:"
    echo "  TypeScript: cd caliber-sdk && npm install && npm run build"
    echo "  Python:     cd sdks/python && pip install -e ."
    echo "  Go:         cd sdks/go && go mod tidy"
    echo "  Elixir:     cd sdks/elixir && mix deps.get && mix compile"
}

# Show usage
usage() {
    echo "CALIBER SDK Generator"
    echo ""
    echo "Usage: $0 [command]"
    echo ""
    echo "Commands:"
    echo "  all         Generate all SDKs (default)"
    echo "  spec        Generate OpenAPI spec only"
    echo "  typescript  Generate TypeScript SDK only"
    echo "  python      Generate Python SDK only"
    echo "  go          Generate Go SDK only"
    echo "  elixir      Generate Elixir SDK only"
    echo "  help        Show this help message"
    echo ""
    echo "Prerequisites:"
    echo "  - Rust toolchain with cargo"
    echo "  - openapi-generator-cli (npm install -g @openapitools/openapi-generator-cli)"
    echo "  - protoc (optional, for gRPC)"
}

# Main
main() {
    local cmd="${1:-all}"

    case "$cmd" in
        all)
            generate_all
            ;;
        spec)
            generate_openapi_spec
            validate_spec
            ;;
        typescript|ts)
            [[ ! -f "$SPEC_FILE" ]] && generate_openapi_spec
            generate_typescript
            ;;
        python|py)
            [[ ! -f "$SPEC_FILE" ]] && generate_openapi_spec
            generate_python
            ;;
        go)
            [[ ! -f "$SPEC_FILE" ]] && generate_openapi_spec
            generate_go
            ;;
        elixir|ex)
            [[ ! -f "$SPEC_FILE" ]] && generate_openapi_spec
            generate_elixir
            ;;
        help|--help|-h)
            usage
            ;;
        *)
            log_error "Unknown command: $cmd"
            usage
            exit 1
            ;;
    esac
}

main "$@"
