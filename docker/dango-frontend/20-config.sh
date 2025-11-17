#!/bin/sh
cat > /usr/share/nginx/html/static/js/config.js <<EOF
window.dango={"chain":{"id":"${CHAIN_ID:-localdango-1}","name":"Local","nativeCoin":"dango","blockExplorer":{"name":"Local Explorer","txPage":"/tx/\${txHash}","accountPage":"/account/\${address}","contractPage":"/contract/\${address}"},"urls":{"indexer":"${INDEXER_URL:-http://localhost:8080}"}},"urls":{"faucetUrl":"${FAUCET_URL:-http://localhost:8082}","questUrl":"${QUEST_URL:-http://localhost:8081}","upUrl":"${UP_URL:-http://localhost:8080/up}"},"banner":"${BANNER}"};
EOF
