#!/bin/ash

genesisFileUrl="https://drive.google.com/uc?export=download&id=19o_2d-OcbHoxwsRQU8ccRh401UxYgcUD"

# Initialize the CometBFT directory
cometbft init

# Open the Tendermint RPC listing port
sed -i 's|laddr = "tcp://127.0.0.1:26657"|laddr = "tcp://0.0.0.0:26657"|g' ~/.cometbft/config/config.toml

# We always use this concensus key for devnets
echo '{
  "address": "8120D21A14941299D4AFC0B78741AEC645BD1431",
  "pub_key": {
    "type": "tendermint/PubKeyEd25519",
    "value": "uNawkPKUb71iZ+OtHk7K+qnWI7FndJEtVdu3wa28qvs="
  },
  "priv_key": {
    "type": "tendermint/PrivKeyEd25519",
    "value": "PnU2Gcd+uQf0vq3J/u1hgbN//uzUsPkQTPFXKX77wrS41rCQ8pRvvWJn460eTsr6qdYjsWd0kS1V27fBrbyq+w=="
  }
}' > ~/.cometbft/config/priv_validator_key.json

# Download genesis file
wget $genesisFileUrl -O ~/.cometbft/config/genesis.json
