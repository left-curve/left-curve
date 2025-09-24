#!/bin/bash

clickhouse-client -q "CREATE DATABASE IF NOT EXISTS {{ clickhouse_database }}" 2>/dev/null ||
clickhouse-client --password="{{ clickhouse_password }}" -q "CREATE DATABASE IF NOT EXISTS {{ clickhouse_database }}"
