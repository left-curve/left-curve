#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/.." && pwd)"

missing=0

check_required() {
  local name="$1"
  local hint="$2"

  if command -v "$name" >/dev/null 2>&1; then
    printf 'ok: %s found at %s\n' "$name" "$(command -v "$name")"
  else
    printf 'missing: %s (%s)\n' "$name" "$hint" >&2
    missing=1
  fi
}

check_optional() {
  local name="$1"
  local hint="$2"

  if command -v "$name" >/dev/null 2>&1; then
    printf 'ok: %s found at %s\n' "$name" "$(command -v "$name")"
  else
    printf 'optional missing: %s (%s)\n' "$name" "$hint"
  fi
}

printf 'SOPS local tool check\n'
printf 'repo: %s\n\n' "$REPO_ROOT"

check_required "sops" "install SOPS before editing or re-encrypting SOPS files"
check_required "age-keygen" "install age before generating the deploy-ci age key"
check_optional "age-plugin-yubikey" "needed for users who create YubiKey-backed age recipients"

printf '\nSOPS repo config\n'
if [ -f "$REPO_ROOT/.sops.yaml" ]; then
  if grep -q 'PLACEHOLDER' "$REPO_ROOT/.sops.yaml"; then
    printf 'not ready: .sops.yaml still contains placeholder recipients\n' >&2
    missing=1
  else
    printf 'ok: .sops.yaml exists and has no placeholder recipients\n'
  fi
else
  printf 'setup needed: .sops.yaml does not exist yet\n'
  missing=1
fi

if [ -f "$REPO_ROOT/.sops.yaml.example" ]; then
  printf 'ok: .sops.yaml.example exists\n'
else
  printf 'missing: .sops.yaml.example is not present\n' >&2
  missing=1
fi

exit "$missing"
