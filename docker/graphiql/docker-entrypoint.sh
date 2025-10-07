#!/bin/sh
set -eu

# Where nginx serves files from
DOCROOT=${NGINX_DOCROOT:-/usr/share/nginx/html}
CONFIG_DIR="$DOCROOT/__/"
CONFIG_FILE="$CONFIG_DIR/config.js"

# Ensure the config directory exists
mkdir -p "$CONFIG_DIR"

# Resolve endpoint (allow override via env)
ENDPOINT=${GRAPHQL_ENDPOINT:-http://localhost:4000/graphql}

# Write runtime config used by index.html
cat > "$CONFIG_FILE" <<EOF
window.GRAPHIQL_CONFIG = {
  endpoint: "${ENDPOINT}"
};
EOF

echo "Wrote runtime config to $CONFIG_FILE (endpoint=$ENDPOINT)"

# Start nginx in foreground
exec nginx -g "daemon off;"

