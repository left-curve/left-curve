#!/bin/sh
TIMESTAMP=$(date +%Y%m%d_%H%M%S)
USER="default"
PASSWORD="{{ clickhouse_password }}"

# Function to backup database if it exists
backup_db() {
    local db_name=$1
    local backup_file="/var/log/clickhouse-server/backups/${db_name}_${TIMESTAMP}.zip"

    echo "Backing up ${db_name}..."
    if docker exec clickhouse clickhouse-client --user=$USER --password=$PASSWORD --query "EXISTS DATABASE ${db_name}" | grep -q "1"; then
        docker exec clickhouse clickhouse-client --user=$USER --password=$PASSWORD --query "BACKUP DATABASE ${db_name} TO File('${backup_file}')" && \
        echo "✓ Backup created: ${backup_file}" || \
        echo "✗ Failed to backup ${db_name}"
    else
        echo "⚠ Database ${db_name} does not exist, skipping"
    fi
}

# Backup both databases
backup_db "testnet_dango_production"
backup_db "devnet_dango_production"

echo "Backup process completed"
