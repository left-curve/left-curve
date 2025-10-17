#!/usr/bin/env bash
set -euo pipefail
project="${1:?usage: $0 <compose-project>}"

ids="$(docker ps -aq --filter "label=com.docker.compose.project=${project}")"
[ -n "$ids" ] || { echo "no containers for ${project}"; exit 1; }

bad=0
# docker Go templates must be wrapped to avoid Jinja
while IFS=, read -r name status state; do
  name="${name#/}"
  if [[ "$status" != *healthy* && "$state" != running ]]; then
    echo "❌ $name -> $status / $state"
    bad=1
  else
    echo "✅ $name -> $status / $state"
  fi
done < <(docker ps --filter "label=com.docker.compose.project=${project}" --format "{% raw %}{{.Names}},{{.Status}},{{.State}}{% endraw %}")
exit $bad
