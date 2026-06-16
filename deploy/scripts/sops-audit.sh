#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)"
DEPLOY_DIR="$(cd -- "$SCRIPT_DIR/.." && pwd)"
REPO_ROOT="$(cd -- "$DEPLOY_DIR/.." && pwd)"

EXPECTED_FILES='deploy/group_vars/all/vault.sops.yml
deploy/vaults/deploy/deploy_key.sops
deploy/group_vars/dango-assistant/vault.sops.yml
deploy/group_vars/hyperlane/vault.sops.yml
deploy/group_vars/perps-bot/vault.sops.yml
deploy/group_vars/points-bot/vault.sops.yml
deploy/host_vars/100.107.248.71/vault.sops.yml
deploy/host_vars/100.122.37.57/main.sops.yml
deploy/host_vars/100.96.253.40/vault.sops.yml
deploy/vaults/debian/debian_key.sops
deploy/vaults/debian/root_vault.sops.yml'

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

count_visible_metadata() {
  local pattern="$1"
  local file="$2"

  grep -Ec "$pattern" "$file" 2>/dev/null || true
}

printf 'SOPS audit (metadata only; no decrypt)\n'
printf 'repo: %s\n\n' "$REPO_ROOT"

status=0
if [ -f "$REPO_ROOT/.sops.yaml" ]; then
  if grep -q 'PLACEHOLDER' "$REPO_ROOT/.sops.yaml"; then
    printf 'config: .sops.yaml present but still contains placeholder recipients\n'
    status=1
  else
    printf 'config: .sops.yaml present with no placeholder recipients\n'
  fi
else
  printf 'config: .sops.yaml missing; scaffold only\n'
  status=1
fi

printf '\nExpected SOPS files\n'
while IFS= read -r rel; do
  [ -n "$rel" ] || continue

  file="$REPO_ROOT/$rel"
  if [ ! -f "$file" ]; then
    printf 'missing  %s\n' "$rel"
    status=1
    continue
  fi

  if grep -q '^[[:space:]]*sops:' "$file" || grep -q '"sops":[[:space:]]*{' "$file"; then
    age_count="$(count_visible_metadata '^[[:space:]]*-[[:space:]]*recipient:|"recipient"[[:space:]]*:' "$file")"
    pgp_count="$(count_visible_metadata '^[[:space:]]*fp:|"fp"[[:space:]]*:' "$file")"
    printf 'present  %s  metadata=yes age_recipients=%s pgp_fps=%s\n' "$rel" "$age_count" "$pgp_count"
  else
    printf 'present  %s  metadata=no\n' "$rel"
    status=1
  fi
done <<EOF
$EXPECTED_FILES
EOF

printf '\nUnexpected SOPS-looking files\n'
unexpected=0
while IFS= read -r file; do
  [ -n "$file" ] || continue
  rel="${file#"$REPO_ROOT"/}"
  if is_expected "$rel"; then
    continue
  fi
  unexpected=1
  printf 'unexpected  %s\n' "$rel"
done <<EOF
$(find "$REPO_ROOT/deploy" -type f \( -name '*.sops.yml' -o -name '*.sops' \) | sort)
EOF

if [ "$unexpected" -eq 0 ]; then
  printf 'none\n'
fi

printf '\nAnsible Vault files still present\n'
vault_count=0
while IFS= read -r file; do
  [ -n "$file" ] || continue
  first_line="$(sed -n '1p' "$file")"
  case "$first_line" in
    '$ANSIBLE_VAULT'*)
      vault_count=$((vault_count + 1))
      printf 'vault  %s\n' "${file#"$REPO_ROOT"/}"
      ;;
  esac
done <<EOF
$(find "$REPO_ROOT/deploy/group_vars" "$REPO_ROOT/deploy/host_vars" "$REPO_ROOT/deploy/vaults" -type f | sort)
EOF

printf 'vault_count=%s\n' "$vault_count"
if [ "$vault_count" -ne 0 ]; then
  status=1
fi

exit "$status"
