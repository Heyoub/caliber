#!/bin/bash
# CALIBER SDK publish helper
#
# Usage:
#   scripts/publish-sdk.sh <version> [--publish]
#   scripts/publish-sdk.sh --publish   (uses latest git tag)
#
# This script generates SDKs with a consistent version and optionally
# publishes them. Publishing is opt-in to avoid accidental releases.

set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
ROOT_DIR="$(dirname "$SCRIPT_DIR")"

PUBLISH=false
VERSION_ARG=""

for arg in "$@"; do
    case "$arg" in
        --publish)
            PUBLISH=true
            ;;
        --help|-h)
            echo "Usage: $0 <version> [--publish]"
            echo "       $0 --publish   # uses latest git tag"
            exit 0
            ;;
        *)
            VERSION_ARG="$arg"
            ;;
    esac
done

if [[ -n "$VERSION_ARG" ]]; then
    SDK_VERSION="$VERSION_ARG"
else
    SDK_VERSION="$(git -C "$ROOT_DIR" describe --tags --abbrev=0 2>/dev/null || true)"
fi

if [[ -z "$SDK_VERSION" ]]; then
    echo "ERROR: No version provided and no git tag found."
    echo "Provide a version like: scripts/publish-sdk.sh 0.1.0"
    exit 1
fi

# Strip leading "v" if provided (e.g., v0.1.0)
SDK_VERSION="${SDK_VERSION#v}"

export SDK_VERSION
echo "[INFO] Generating SDKs with version ${SDK_VERSION}"

"$ROOT_DIR/scripts/generate-sdk.sh"

if [[ "$PUBLISH" == "true" ]]; then
    echo "[INFO] Publishing TypeScript SDK to npm..."
    (cd "$ROOT_DIR/sdks/typescript" && npm publish --access public)

    echo "[INFO] Publishing Python SDK to PyPI..."
    (cd "$ROOT_DIR/sdks/python" && python -m build && twine upload dist/*)

    echo "[INFO] Go SDK publishing is tag-based; ensure git tag v${SDK_VERSION} is pushed."
    echo "[INFO] Publishing Elixir SDK to Hex..."
    (cd "$ROOT_DIR/sdks/elixir" && mix hex.publish --yes)
else
    echo "[INFO] SDKs generated under $ROOT_DIR/sdks."
    echo "[INFO] To publish, re-run with: scripts/publish-sdk.sh ${SDK_VERSION} --publish"
fi
