import { createConfig, devnet, graphql, passkey, session } from "@left-curve/store";

import type { Config } from "@left-curve/store/types";
const dango = devnet;

export const GRAPHQL_URI =
  import.meta.env.PUBLIC_ENVIRONMENT === "local"
    ? import.meta.env.PUBLIC_GRAPHQL_URI
    : dango.urls.indexer;

export const coins = {
  "hyp/all/btc": {
    type: "native",
    name: "Bitcoin",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg",
    symbol: "BTC",
    denom: "hyp/all/btc",
    decimals: 6,
    coingeckoId: "bitcoin",
  },
  "hyp/all/eth": {
    type: "native",
    name: "Ether",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/ethereum/images/eth.svg",
    symbol: "ETH",
    denom: "hyp/all/eth",
    decimals: 6,
    coingeckoId: "ethereum",
  },
  "hyp/all/xrp": {
    type: "native",
    name: "Ripple",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/xrpl/images/xrp.svg",
    symbol: "XRP",
    denom: "hyp/all/xrp",
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
  "hyp/all/sol": {
    type: "native",
    name: "Solana",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/solana/images/sol.svg",
    symbol: "SOL",
    denom: "hyp/all/sol",
    decimals: 6,
    coingeckoId: "solana",
  },
} as const;

export const coinsBySymbol = Object.values(coins).reduce((acc, coin) => {
  acc[coin.symbol] = coin;
  return acc;
}, Object.create({}));

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain: dango,
  transport: graphql(GRAPHQL_URI, { batch: true }),
  coins,
  connectors: [passkey(), session()],
});
