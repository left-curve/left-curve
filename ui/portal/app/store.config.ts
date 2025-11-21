import { createConfig, graphql, passkey, devnet, createStorage } from "@left-curve/store";

import type { Config } from "@left-curve/store/types";
import { createMMKVStorage } from "./storage.config";

const chain = devnet;

const coins = {
  dango: {
    type: "native",
    name: "Dango",
    logoURI: "/DGX.svg",
    symbol: "DGX",
    denom: "dango",
    decimals: 6,
  },
  "bridge/btc": {
    type: "native",
    name: "Bitcoin",
    logoURI: "/images/coins/bitcoin.svg",
    symbol: "BTC",
    denom: "bridge/btc",
    decimals: 8,
    coingeckoId: "bitcoin",
  },
  "bridge/eth": {
    type: "native",
    name: "Ether",
    logoURI: "/images/coins/eth.svg",
    symbol: "ETH",
    denom: "bridge/eth",
    decimals: 18,
    coingeckoId: "ethereum",
  },
  "bridge/xrp": {
    type: "native",
    name: "XRP",
    logoURI: "/images/coins/xrp.svg",
    symbol: "XRP",
    denom: "bridge/xrp",
    decimals: 6,
    coingeckoId: "ripple",
  },
  "bridge/usdc": {
    type: "native",
    name: "USD Coin",
    logoURI: "/images/coins/usdc.svg",
    symbol: "USDC",
    denom: "bridge/usdc",
    decimals: 6,
    coingeckoId: "usd-coin",
  },
  "bridge/sol": {
    type: "native",
    name: "Solana",
    logoURI: "/images/coins/sol.svg",
    symbol: "SOL",
    denom: "bridge/sol",
    decimals: 9,
    coingeckoId: "solana",
  },
} as const;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: false,
  chain,
  transport: graphql(chain.urls.indexer, { batch: true }),
  coins,
  storage: createStorage({ storage: createMMKVStorage() }),
  connectors: [passkey()],
});
