#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

DEBIAN_KEY_FILE="$DEPLOY_DIR/vaults/debian/debian_key.vault"
DEBIAN_PASSWORD_SCRIPT="$SCRIPT_DIR/debian-password.sh"

ansible-vault view "$DEBIAN_KEY_FILE" --vault-id debian@"$DEBIAN_PASSWORD_SCRIPT"
