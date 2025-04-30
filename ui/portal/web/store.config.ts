import { createConfig, devnet, graphql, passkey, session } from "@left-curve/store";

import type { Config } from "@left-curve/store/types";

const dango = devnet;

const GRAPHQL_URI =
  import.meta.env.PUBLIC_ENVIRONMENT === "local"
    ? import.meta.env.PUBLIC_GRAPHQL_URI
    : dango.urls.indexer;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain: dango,
  transport: graphql(GRAPHQL_URI, { batch: true }),
  coins: {
    [dango.id]: {
      "hyp/btc/btc": {
        type: "native",
        name: "Bitcoin",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg",
        symbol: "BTC",
        denom: "hyp/btc/btc",
        decimals: 6,
        coingeckoId: "bitcoin",
      },
      "hyp/eth/eth": {
        type: "native",
        name: "Ether",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/ethereum/images/eth.svg",
        symbol: "ETH",
        denom: "hyp/eth/eth",
        decimals: 6,
        coingeckoId: "ethereum",
      },
      "hyp/xrp/xrp": {
        type: "native",
        name: "Ripple",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/xrpl/images/xrp.svg",
        symbol: "XRP",
        denom: "hyp/xrp/xrp",
        decimals: 6,
        coingeckoId: "ripple",
      },
      "hyp/eth/usdc": {
        type: "alloyed",
        name: "Ethereum USD Circle",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
        symbol: "USDC",
        denom: "hyp/eth/usdc",
        decimals: 6,
        coingeckoId: "usd-coin",
      },
      "hyp/sol/sol": {
        type: "native",
        name: "Solana",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/solana/images/sol.svg",
        symbol: "SOL",
        denom: "hyp/sol/sol",
        decimals: 6,
        coingeckoId: "solana",
      },
    },
  },
  connectors: [passkey(), session()],
});
