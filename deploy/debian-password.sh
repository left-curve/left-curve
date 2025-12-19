#!/bin/bash

if [ -n "$ANSIBLE_DEBIAN_PASSWORD" ]; then
  # CI/GitHub Actions - use environment variable
  echo "$ANSIBLE_DEBIAN_PASSWORD"
elif command -v security >/dev/null 2>&1; then
  # macOS - use Keychain. If missing, return empty so non-debian playbooks still run.
  security find-generic-password -a ansible -s ansible-debian/default -w 2>/dev/null || true
else
  # No password source available; leave empty so non-debian playbooks still run.
  exit 0
fi
