#!/bin/bash

echo "DB_HOST: $DB_HOST"
echo "DB_PORT: $DB_PORT"
echo "DB_USER: $DB_USER"
echo "DB_NAME: $DB_NAME"

# Wait until PostgreSQL is ready.
until pg_isready -h $DB_HOST -p $DB_PORT -U $DB_USER; do
  echo "Waiting for PostgreSQL to become available at postgres://$DB_USER@$DB_HOST:$DB_PORT..."
  sleep 1
done

# Create the database if it doesn't exist.
exists=$(psql -h $DB_HOST -p $DB_PORT -U $DB_USER -d postgres -t -A -c "SELECT 1 FROM pg_database WHERE datname='$DB_NAME'")
if [ "$exists" = "1" ]; then
  echo "Database $DB_NAME already exists."
else
  echo "Database $DB_NAME does not exist. Creating..."
  createdb -h $DB_HOST -p $DB_PORT -U $DB_USER $DB_NAME
  if [ $? -ne 0 ]; then
    echo "Failed to create database $DB_NAME."
    exit 1
  fi
  echo "Database $DB_NAME successfully created."
fi

# Run dango.
# This assumes a config file has been mounted to `~/.dango/config/app.toml`.
dango start
