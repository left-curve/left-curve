#!/usr/bin/env bash
set -euo pipefail
project="${1:?usage: $0 <compose-project>}"

mapfile -t lines < <(docker ps --filter "label=com.docker.compose.project=${project}" \
  --format "{{.Names}},{{.Status}},{{.State}}")

if [ "${#lines[@]}" -eq 0 ]; then
  echo "no containers for ${project}"
  exit 1
fi

bad=0
for line in "${lines[@]}"; do
  IFS=, read -r name status state <<<"$line"
  name="${name#/}"

  # Consider healthy ok; any 'unhealthy', 'exited', or 'dead' is fail
  if [[ "$status" =~ "\(unhealthy\)" ]] || [[ "$state" != "running" ]]; then
    echo "❌ $name -> $status / $state"
    bad=1
  else
    echo "✅ $name -> $status / $state"
  fi
done

exit $bad
