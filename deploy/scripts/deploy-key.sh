#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"

VAULT_KEY_FILE="$DEPLOY_DIR/group_vars/all/deploy_key.vault"
VAULT_PASSWORD_SCRIPT="$SCRIPT_DIR/vault-password.sh"

ansible-vault view "$VAULT_KEY_FILE" --vault-password-file "$VAULT_PASSWORD_SCRIPT"
