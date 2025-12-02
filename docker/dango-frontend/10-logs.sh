#!/bin/sh
set -eu

# Generate logging configuration based on $NGINX_JSON_LOGS
LOG_CONF=/etc/nginx/conf.d/logs.conf

cat > "$LOG_CONF" <<'EOF'
# Logging config generated at container start

log_format main '$remote_addr - $remote_user [$time_local] "$request" '
                '$status $body_bytes_sent "$http_referer" '
                '"$http_user_agent" "$http_x_forwarded_for"';

log_format json_combined escape=json '{'
    '"timestamp":"$time_iso8601",'
    '"remote_addr":"$remote_addr",'
    '"remote_user":"$remote_user",'
    '"request":"$request",'
    '"status":"$status",'
    '"body_bytes_sent":"$body_bytes_sent",'
    '"request_time":"$request_time",'
    '"http_referer":"$http_referer",'
    '"http_user_agent":"$http_user_agent",'
    '"http_x_forwarded_for":"$http_x_forwarded_for",'
    '"host":"$host",'
    '"uri":"$uri",'
    '"method":"$request_method",'
    '"protocol":"$server_protocol"'
'}';
EOF

case "${NGINX_JSON_LOGS:-}" in
  1|true|TRUE|yes|YES|on|ON|json|JSON)
    echo "access_log /dev/stdout json_combined;" >> "$LOG_CONF"
    ;;
  *)
    echo "access_log /dev/stdout main;" >> "$LOG_CONF"
    ;;
esac

