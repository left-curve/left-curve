#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/.." && pwd)"

EXPECTED_FILES='deploy/group_vars/all/vault.sops.json
deploy/vaults/deploy/deploy_key.sops
deploy/group_vars/dango-assistant/vault.sops.json
deploy/group_vars/hyperlane/vault.sops.json
deploy/group_vars/perps-bot/vault.sops.json
deploy/group_vars/points-bot/vault.sops.json
deploy/host_vars/100.107.248.71/vault.sops.json
deploy/host_vars/100.122.37.57/main.sops.json
deploy/host_vars/100.96.253.40/vault.sops.json
deploy/vaults/debian/debian_key.sops
deploy/vaults/debian/root_vault.sops.json'

usage() {
  cat <<'EOF'
Usage:
  sops-reencrypt.sh [--dry-run] [--list] [PATH ...]

Re-encrypt expected SOPS files with recipients from .sops.yaml.
With no PATH arguments, all existing expected files are processed.

This script only operates on the deploy *.sops.yml/*.sops.json/*.sops paths.
EOF
}

is_expected() {
  local candidate="$1"
  local expected

  while IFS= read -r expected; do
    [ "$candidate" = "$expected" ] && return 0
  done <<EOF
$EXPECTED_FILES
EOF

  return 1
}

normalize_path() {
  local input="$1"
  local rel="$input"

  case "$rel" in
    "$REPO_ROOT"/*) rel="${rel#"$REPO_ROOT"/}" ;;
    ./*) rel="${rel#./}" ;;
  esac

  case "$rel" in
    ""|/*|..|../*|*/..|*/../*|*"/./"*|./*)
      printf 'error: invalid path: %s\n' "$input" >&2
      exit 2
      ;;
  esac

  printf '%s\n' "$rel"
}

require_tools() {
  if [ ! -f "$REPO_ROOT/.sops.yaml" ]; then
    echo "error: .sops.yaml is missing" >&2
    exit 1
  fi

  if grep -q 'PLACEHOLDER' "$REPO_ROOT/.sops.yaml"; then
    echo "error: .sops.yaml still contains placeholder recipients" >&2
    exit 1
  fi

  if ! command -v sops >/dev/null 2>&1; then
    echo "error: sops is not installed or not on PATH" >&2
    exit 1
  fi
}

dry_run=0
explicit_targets=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --list)
      printf '%s\n' "$EXPECTED_FILES"
      exit 0
      ;;
    --*)
      printf 'error: unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
    *)
      rel="$(normalize_path "$1")"
      explicit_targets="${explicit_targets}${rel}
"
      shift
      ;;
  esac
done

require_tools

targets=""
if [ -n "$explicit_targets" ]; then
  targets="$explicit_targets"
else
  while IFS= read -r rel; do
    [ -f "$REPO_ROOT/$rel" ] || continue
    targets="${targets}${rel}
"
  done <<EOF
$EXPECTED_FILES
EOF
fi

if [ -z "$targets" ]; then
  echo "error: no expected SOPS files exist yet; migration has not created them" >&2
  exit 1
fi

status=0
while IFS= read -r rel; do
  [ -n "$rel" ] || continue

  if ! is_expected "$rel"; then
    printf 'error: refusing unexpected path: %s\n' "$rel" >&2
    status=1
    continue
  fi

  case "$rel" in
    *.sops.yml|*.sops.json|*.sops) ;;
    *)
      printf 'error: refusing non-SOPS path: %s\n' "$rel" >&2
      status=1
      continue
      ;;
  esac

  if [ ! -f "$REPO_ROOT/$rel" ]; then
    printf 'error: expected SOPS file does not exist: %s\n' "$rel" >&2
    status=1
    continue
  fi

  if ! grep -q '"sops":[[:space:]]*{' "$REPO_ROOT/$rel" && ! grep -q '^[[:space:]]*sops:' "$REPO_ROOT/$rel"; then
    printf 'error: file has no visible SOPS metadata: %s\n' "$rel" >&2
    status=1
    continue
  fi

  if [ "$dry_run" -eq 1 ]; then
    printf 'would update SOPS recipients: %s\n' "$rel"
  else
    printf 'updating SOPS recipients: %s\n' "$rel"
    (cd "$REPO_ROOT" && sops updatekeys --yes "$rel")
  fi
done <<EOF
$targets
EOF

exit "$status"
