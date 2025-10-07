#!/bin/sh
set -eu

# Where nginx serves files from
DOCROOT=${NGINX_DOCROOT:-/usr/share/nginx/html}
CONFIG_DIR="$DOCROOT/__/"
CONFIG_FILE="$CONFIG_DIR/config.js"

# Ensure the config directory exists
mkdir -p "$CONFIG_DIR"

# Resolve endpoint only if provided; otherwise leave empty to auto-detect
ENDPOINT=${GRAPHQL_ENDPOINT:-}

# Write runtime config used by index.html
if [ -n "$ENDPOINT" ]; then
  cat > "$CONFIG_FILE" <<EOF
window.GRAPHIQL_CONFIG = {
  endpoint: "${ENDPOINT}"
};
EOF
  echo "Wrote runtime config to $CONFIG_FILE (endpoint=$ENDPOINT)"
else
  echo "window.GRAPHIQL_CONFIG = {};" > "$CONFIG_FILE"
  echo "Wrote runtime config to $CONFIG_FILE (no endpoint; app will auto-detect current path)"
fi

# Start nginx in foreground
exec nginx -g "daemon off;"
