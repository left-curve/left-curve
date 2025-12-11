# How to run the validator and relayer

Based on the agent to run, copy the `.env.example` file and edit the variables.
Name the file to the network name, e.g. `testnet.env`.

``` bash
cd docker/hyperlane/relayer
cp .env.example testnet.env
```

then run the docker compose file:

``` bash
NETWORK=testnet docker compose up
```
