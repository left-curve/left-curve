#!/bin/bash

if [ -n "$ANSIBLE_VAULT_PASSWORD" ]; then
  # Use cached environment variable (set by just add-passwords or CI)
  echo "$ANSIBLE_VAULT_PASSWORD"
elif command -v pass >/dev/null 2>&1; then
  # Use pass (passwordstore.org)
  pass show dango/deploy-vault
elif command -v security >/dev/null 2>&1; then
  # macOS - use Keychain
  security find-generic-password -a ansible -s ansible-vault/default -w
else
  echo "Error: No vault password source available" >&2
  exit 1
fi
