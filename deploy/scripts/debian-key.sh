#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

DEBIAN_KEY_FILE="$DEPLOY_DIR/vaults/debian/debian_key.sops"

sops --decrypt --input-type binary --output-type binary "$DEBIAN_KEY_FILE"
