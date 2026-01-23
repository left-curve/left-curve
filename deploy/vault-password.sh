#!/bin/bash

if [ -n "$ANSIBLE_VAULT_PASSWORD" ]; then
  # CI/GitHub Actions - use environment variable
  echo "$ANSIBLE_VAULT_PASSWORD"
elif command -v pass >/dev/null 2>&1 && pass show dango/deploy-vault >/dev/null 2>&1; then
  # Use pass (passwordstore.org) if available
  pass show dango/deploy-vault
elif command -v security >/dev/null 2>&1; then
  # macOS - use Keychain
  security find-generic-password -a ansible -s ansible-vault/default -w
else
  echo "Error: No vault password source available" >&2
  exit 1
fi
