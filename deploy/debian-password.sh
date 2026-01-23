#!/bin/bash

if [ -n "$ANSIBLE_DEBIAN_PASSWORD" ]; then
  # CI/GitHub Actions - use environment variable
  echo "$ANSIBLE_DEBIAN_PASSWORD"
elif command -v pass >/dev/null 2>&1 && pass show dango/debian-vault >/dev/null 2>&1; then
  # Use pass (passwordstore.org) if available
  pass show dango/debian-vault
elif command -v security >/dev/null 2>&1; then
  # macOS - use Keychain. If missing, return placeholder so non-debian playbooks still run.
  security find-generic-password -a ansible -s ansible-debian/default -w 2>/dev/null || echo "ANSIBLE_DEBIAN_PASSWORD_NOT_SET"
else
  # No password source available; return placeholder so non-debian playbooks still run.
  echo "ANSIBLE_DEBIAN_PASSWORD_NOT_SET"
fi
