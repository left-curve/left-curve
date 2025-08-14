#!/bin/sh
cat > /usr/share/nginx/html/static/js/config.js <<EOF
window.dango={"chain":{"id":"localdango-1","name":"Local","nativeCoin":"dango","blockExplorer":{"name":"Local Explorer","txPage":"/tx/\${txHash}","accountPage":"/account/\${address}","contractPage":"/contract/\${address}"},"urls":{"indexer":"${INDEXER_URL:-http://localhost:8080/graphql}"}},"urls":{"faucetUrl":"${FAUCET_URL:-http://localhost:8082/mint}","questUrl":"${QUEST_URL:-http://localhost:8081/check_username}","upUrl":"${UP_URL:-http://localhost:8080/up}"}};
EOF
