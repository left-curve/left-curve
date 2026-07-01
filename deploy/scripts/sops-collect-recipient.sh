#!/usr/bin/env bash
set -euo pipefail

usage() {
  cat <<'EOF'
Usage:
  sops-collect-recipient.sh --user NAME [--group routine|root] [RECIPIENT]
  sops-collect-recipient.sh --user NAME [--group routine|root] < recipient.txt

Collect and validate a public age/YubiKey recipient, then print a reviewable
.sops.yaml snippet. This script does not write to .sops.yaml and does not
accept private age keys.
EOF
}

user=""
group="routine"
recipient=""

while [ "$#" -gt 0 ]; do
  case "$1" in
    -h|--help)
      usage
      exit 0
      ;;
    --user)
      if [ "$#" -lt 2 ]; then
        echo "error: --user requires a value" >&2
        exit 2
      fi
      user="$2"
      shift 2
      ;;
    --group)
      if [ "$#" -lt 2 ]; then
        echo "error: --group requires a value" >&2
        exit 2
      fi
      group="$2"
      shift 2
      ;;
    --*)
      printf 'error: unknown option: %s\n' "$1" >&2
      usage >&2
      exit 2
      ;;
    *)
      if [ -n "$recipient" ]; then
        echo "error: only one recipient may be supplied" >&2
        exit 2
      fi
      recipient="$1"
      shift
      ;;
  esac
done

if [ -z "$user" ]; then
  echo "error: --user is required" >&2
  exit 2
fi

case "$user" in
  *[!A-Za-z0-9._-]*|"")
    echo "error: --user may contain only letters, numbers, dots, underscores, and dashes" >&2
    exit 2
    ;;
esac

case "$group" in
  routine|root) ;;
  *)
    echo "error: --group must be routine or root" >&2
    exit 2
    ;;
esac

if [ -z "$recipient" ]; then
  if [ -t 0 ]; then
    printf 'Paste the public age recipient for %s: ' "$user" >&2
  fi
  IFS= read -r recipient || true
fi

recipient="${recipient#"${recipient%%[![:space:]]*}"}"
recipient="${recipient%"${recipient##*[![:space:]]}"}"

if [ -z "$recipient" ]; then
  echo "error: no recipient supplied" >&2
  exit 2
fi

case "$recipient" in
  *AGE-SECRET-KEY*|*"BEGIN OPENSSH PRIVATE KEY"*|*"PRIVATE KEY"*)
    echo "error: input looks like a private key; expected a public age recipient" >&2
    exit 2
    ;;
esac

case "$recipient" in
  *[[:space:]]*)
    echo "error: recipient must be a single token with no whitespace" >&2
    exit 2
    ;;
esac

case "$recipient" in
  age1*) ;;
  *)
    echo "error: recipient must start with age1" >&2
    exit 2
    ;;
esac

cat <<EOF
# Public SOPS recipient collected locally.
# Review before copying into .sops.yaml.
# user: $user
# group: $group
- $recipient
EOF
