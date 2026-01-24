#!/bin/bash
# CALIBER SDK publish helper
#
# Usage:
#   scripts/publish-sdk.sh [<version>] [--publish] [--all|--typescript-only|--python-only|--go-only|--elixir-only]
#   scripts/publish-sdk.sh --publish --all            (default)
#   scripts/publish-sdk.sh --publish --typescript-only
#
# This script generates SDKs with a consistent version and optionally
# publishes them. Publishing is opt-in to avoid accidental releases.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

PUBLISH=false
MODE="all"
VERSION_ARG=""

usage() {
    echo "Usage: $0 [<version>] [--publish] [--all|--typescript-only|--python-only|--go-only|--elixir-only]"
    echo "       $0 --publish --all            # default"
    echo "       $0 --publish --typescript-only"
}

get_workspace_version() {
    awk '
        BEGIN { in_workspace = 0 }
        /^\[workspace\.package\]/ { in_workspace = 1; next }
        /^\[/ { if ($0 != "[workspace.package]") in_workspace = 0 }
        in_workspace && $1 == "version" { gsub(/\"/, "", $3); print $3; exit }
    ' "$ROOT_DIR/Cargo.toml"
}

while [[ $# -gt 0 ]]; do
    case "$1" in
        --publish)
            PUBLISH=true
            shift
            ;;
        --all)
            MODE="all"
            shift
            ;;
        --typescript-only|--ts-only)
            MODE="typescript"
            shift
            ;;
        --python-only|--py-only)
            MODE="python"
            shift
            ;;
        --go-only)
            MODE="go"
            shift
            ;;
        --elixir-only|--ex-only)
            MODE="elixir"
            shift
            ;;
        --help|-h)
            usage
            exit 0
            ;;
        *)
            if [[ -z "$VERSION_ARG" ]]; then
                VERSION_ARG="$1"
                shift
            else
                echo "ERROR: Unknown argument '$1'"
                usage
                exit 1
            fi
            ;;
    esac
done

if [[ -n "$VERSION_ARG" ]]; then
    SDK_VERSION="$VERSION_ARG"
else
    SDK_VERSION="$(get_workspace_version)"
fi

if [[ -z "$SDK_VERSION" ]]; then
    echo "ERROR: No version provided and workspace version not found."
    echo "Provide a version like: scripts/publish-sdk.sh 0.4.3"
    exit 1
fi

# Strip leading "v" if provided (e.g., v0.1.0)
SDK_VERSION="${SDK_VERSION#v}"

export SDK_VERSION
echo "[INFO] Generating SDKs with version ${SDK_VERSION}"

"$ROOT_DIR/scripts/generate-sdk.sh" "$MODE"

if [[ "$PUBLISH" == "true" ]]; then
    if [[ "$MODE" == "all" || "$MODE" == "typescript" ]]; then
        echo "[INFO] Publishing TypeScript SDK (@caliber-run/sdk) to npm..."
        (cd "$ROOT_DIR/caliber-sdk" && npm publish --access public)
    fi

    if [[ "$MODE" == "all" || "$MODE" == "python" ]]; then
        echo "[INFO] Publishing Python SDK to PyPI..."
        (cd "$ROOT_DIR/sdks/python" && python -m build && twine upload dist/*)
    fi

    if [[ "$MODE" == "all" || "$MODE" == "go" ]]; then
        echo "[INFO] Go SDK publishing is tag-based; ensure git tag v${SDK_VERSION} is pushed."
    fi

    if [[ "$MODE" == "all" || "$MODE" == "elixir" ]]; then
        echo "[INFO] Publishing Elixir SDK to Hex..."
        (cd "$ROOT_DIR/sdks/elixir" && mix hex.publish --yes)
    fi
else
    echo "[INFO] SDKs generated:"
    echo "         - TypeScript: $ROOT_DIR/caliber-sdk (@caliber-run/sdk)"
    echo "         - Python:     $ROOT_DIR/sdks/python"
    echo "         - Go:         $ROOT_DIR/sdks/go"
    echo "         - Elixir:     $ROOT_DIR/sdks/elixir"
    echo "[INFO] To publish, re-run with: scripts/publish-sdk.sh ${SDK_VERSION} --publish"
fi
