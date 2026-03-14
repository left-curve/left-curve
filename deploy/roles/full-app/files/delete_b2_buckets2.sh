#!/bin/sh

# Clean up stale Backblaze B2 buckets from PR and environment deployments.
#
# Bucket naming: dango-{network}-{timestamp}
#   - PR buckets:      dango-pr-1414-20251205133239
#   - Devnet buckets:  dango-devnet-20251203112135
#   - Testnet buckets: dango-testnet-20251218143900
#
# Safety guards:
#   1. Only targets buckets matching "dango-*" (skips unrelated buckets).
#   2. Only deletes if lifecycle rules are set (confirms delete-full-app.yml
#      already marked the bucket for cleanup).
#   3. Only deletes if fileCount == 0 (B2 finished draining files via the
#      lifecycle rules, which takes ~2 days).
#   4. For persistent environments (devnet/testnet), always keeps the most
#      recent bucket per network prefix (the active deployment).

set -e

# b2 is installed via pipx in ~/.local/bin
export PATH="$HOME/.local/bin:$PATH"

buckets_json=$(b2 bucket list --json)

# Extract dango-pr-* and dango-devnet-* bucket names (skip testnet/mainnet for now).
all_buckets=$(echo "$buckets_json" \
  | jq -r '.[] | select(.bucketName | test("^dango-(pr-|devnet-)")) | .bucketName' \
  | sort)

if [ -z "$all_buckets" ]; then
  echo "No dango-* buckets found."
  exit 0
fi

# Build a set of "latest bucket per network prefix" to protect.
# Network prefix = everything before the last -YYYYMMDDHHMMSS timestamp.
# e.g. dango-pr-1414-20251205133239 → prefix: dango-pr-1414
#      dango-devnet-20251203112135   → prefix: dango-devnet
#
# Since buckets are sorted, the last one per prefix is the most recent.
protected=""
prev_prefix=""
prev_bucket=""
for b in $all_buckets; do
  # Strip trailing -YYYYMMDDHHMMSS (14 digits)
  prefix=$(echo "$b" | sed 's/-[0-9]\{14\}$//')
  if [ "$prefix" != "$prev_prefix" ] && [ -n "$prev_bucket" ]; then
    protected="$protected $prev_bucket"
  fi
  prev_prefix="$prefix"
  prev_bucket="$b"
done
# Don't forget the last group
if [ -n "$prev_bucket" ]; then
  protected="$protected $prev_bucket"
fi

echo "=== Protected (latest per network) ==="
for p in $protected; do
  echo "  PROTECTED: $p"
done
echo ""

echo "=== Bucket analysis ==="
for b in $all_buckets; do
  echo "Checking $b..."

  # Never delete the latest bucket per network prefix
  case " $protected " in
    *" $b "*) echo "  SKIP (latest for its network — protected)"
              continue ;;
  esac

  # timeout after 5s → assume non-empty
  json=$(timeout 5 b2 bucket get --show-size "$b" 2>/dev/null) || true

  if [ -z "$json" ]; then
    echo "  SKIP (slow or error)"
    continue
  fi

  # Only delete buckets that have been marked for cleanup
  rules=$(echo "$json" | jq '.lifecycleRules | length')
  if [ "$rules" -eq 0 ]; then
    echo "  SKIP (no lifecycle rules — not marked for deletion)"
    continue
  fi

  count=$(echo "$json" | jq '.fileCount')

  if [ "$count" -eq 0 ]; then
    echo "  WOULD DELETE (empty, lifecycle rules active)"
    # TODO: uncomment to enable actual deletion
    # b2 bucket delete "$b"
  else
    echo "  KEEP (count=$count, waiting for lifecycle to clear files)"
  fi
done

echo ""
echo "=== Dry run complete — no buckets were deleted ==="
