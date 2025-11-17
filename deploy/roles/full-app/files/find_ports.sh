#!/bin/bash
ports=()
start_port=${1:-30000}
count=${2:-10}
max_port=$((start_port + 10000))

for port in $(seq $start_port $max_port); do
  if ! ss -tuln | grep -q ":$port "; then
    ports+=($port)
    if [ ${#ports[@]} -eq $count ]; then
      break
    fi
  fi
done

echo "${ports[@]}"
