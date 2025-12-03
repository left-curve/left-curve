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
    '"level":"info",'
    '"component":"nginx",'
    '"service":"dango-frontend",'
    '"timestamp":"$time_iso8601",'
    '"remote_addr":"$remote_addr",'
    '"remote_user":"$remote_user",'
    '"request":"$request",'
    '"request_uri":"$request_uri",'
    '"args":"$args",'
    '"status":"$status",'
    '"body_bytes_sent":"$body_bytes_sent",'
    '"bytes_sent":"$bytes_sent",'
    '"request_time":"$request_time",'
    '"http_referer":"$http_referer",'
    '"http_user_agent":"$http_user_agent",'
    '"http_x_forwarded_for":"$http_x_forwarded_for",'
    '"http_x_forwarded_proto":"$http_x_forwarded_proto",'
    '"http_x_forwarded_host":"$http_x_forwarded_host",'
    '"x_request_id":"$http_x_request_id",'
    '"host":"$host",'
    '"server_name":"$server_name",'
    '"server_port":"$server_port",'
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
