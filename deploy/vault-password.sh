#!/bin/bash

if [ -n "$ANSIBLE_VAULT_PASSWORD" ]; then
  # CI/GitHub Actions - use environment variable
  echo "$ANSIBLE_VAULT_PASSWORD"
elif command -v security >/dev/null 2>&1; then
  # macOS - use Keychain
  security find-generic-password -a ansible -s ansible-vault/default -w
else
  # Need to find some for Linux
  echo "Error: No vault password source available" >&2
  exit 1
fi
