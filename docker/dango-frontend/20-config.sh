#!/bin/sh

csv_to_json_array() {
  # Convert a comma-separated string into a JSON array of trimmed strings.
  # Example: " points,foo ,, bar " -> ["points","foo","bar"]
  printf '%s' "${1:-}" | awk -F',' '
    BEGIN { printf "[" }
    {
      for (i = 1; i <= NF; i++) {
        value = $i
        gsub(/^[[:space:]]+|[[:space:]]+$/, "", value)
        if (value == "") continue
        gsub(/\\/, "\\\\", value)
        gsub(/"/, "\\\"", value)
        if (count++) printf ","
        printf "\"%s\"", value
      }
    }
    END { printf "]" }
  '
}

disabled_features_json="$(csv_to_json_array "${FRONTEND_DISABLED_FEATURES:-}")"

cat > /usr/share/nginx/html/static/js/config.js <<EOF
window.dango={"chain":{"id":"${CHAIN_ID:-localdango-1}","name":"Local","nativeCoin":"dango","blockExplorer":{"name":"Local Explorer","txPage":"/tx/\${txHash}","accountPage":"/account/\${address}","contractPage":"/contract/\${address}"},"urls":{"indexer":"${INDEXER_URL:-http://localhost:8080}"}},"urls":{"faucetUrl":"${FAUCET_URL:-http://localhost:8082/mint}","questUrl":"${QUEST_URL:-http://localhost:8081/check_username}","upUrl":"${UP_URL:-http://localhost:8080/up}"},"banner":"${BANNER}","disabledFeatures":${disabled_features_json}};
EOF
