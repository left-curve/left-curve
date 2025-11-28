#!/usr/bin/env bash
set -euo pipefail

VAULT_KEY_FILE="group_vars/all/deploy_key.vault"
VAULT_PASSWORD_SCRIPT="vault-password.sh"

ansible-vault view "$VAULT_KEY_FILE" --vault-password-file "$VAULT_PASSWORD_SCRIPT"
