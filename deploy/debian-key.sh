#!/usr/bin/env bash
set -euo pipefail

DEBIAN_KEY_FILE="group_vars/debian/debian_key.vault"
DEBIAN_PASSWORD_SCRIPT="debian-password.sh"

ansible-vault view "$DEBIAN_KEY_FILE" --vault-id debian@"$DEBIAN_PASSWORD_SCRIPT"
