#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

DEPLOY_KEY_FILE="$DEPLOY_DIR/group_vars/all/deploy_key.sops"

sops --decrypt --input-type binary --output-type binary "$DEPLOY_KEY_FILE"
