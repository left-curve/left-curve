#!/bin/bash

# This script deploys the compiled Hyperlane Warp Route (HWR) contracts to the Solana network.

echo $SOLANA_MNEMONIC

# Convert mnemonic to solana keypair that we can use with the Solana CLI
uv run mnemonic-to-keypair "$SOLANA_MNEMONIC"
