#!/bin/sh

b2 bucket list --json \
  | jq -r '.[].bucketName' \
  | while read -r b; do
      echo "Checking $b..."

      # timeout after 5s → assume non-empty
      json=$(timeout 5 b2 bucket get --show-size "$b" 2>/dev/null)

      if [ $? -ne 0 ] || [ -z "$json" ]; then
        echo "  SKIP (slow or error)"
        continue
      fi

      count=$(echo "$json" | jq '.fileCount')

      if [ "$count" -eq 0 ]; then
        echo "  Deleting empty bucket"
        b2 bucket delete "$b"
      else
        echo "  KEEP (count=$count)"
      fi
    done
