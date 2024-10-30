#!/bin/bash

# Define an array of invariants
invariants=(
  "everyNodesParentIsInTheTreeInv"
  "nodeAtCommonPrefixInv"
  "hashInv"
  "noInternalChildInv"
  "allInternalNodesHaveChildrenInv"
  "densityInv"
  "versionInv"
  "orphansInNoTreeInv"
)

# Check if an index argument is provided
if [ -z "$1" ]; then
  echo "Usage: $0 <index>"
  exit 1
fi

# Get the index from the command line
index=$1

# Validate that the index is within range
if [ "$index" -lt 0 ] || [ "$index" -ge "${#invariants[@]}" ]; then
  echo "Error: Index out of range. Please use an index between 0 and $((${#invariants[@]} - 1))."
  exit 1
fi

# Select the invariant based on the index
invariant="${invariants[$index]}"

# Other parameters
step="step_super_simple"
max_steps=5
max_samples=1000

# Run the command with the selected invariant
quint run apply_state_machine.qnt --invariant="$invariant" --step="$step" --max-steps="$max_steps" --max-samples="$max_samples"
