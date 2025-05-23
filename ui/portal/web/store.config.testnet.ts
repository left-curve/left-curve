import { createConfig, graphql, passkey, session, testnet } from "@left-curve/store";

import type { Config } from "@left-curve/store/types";

const chain = testnet;

export const coins = {
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
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg",
    symbol: "BTC",
    denom: "bridge/btc",
    decimals: 8,
    coingeckoId: "bitcoin",
  },
  "bridge/eth": {
    type: "native",
    name: "Ether",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/ethereum/images/eth.svg",
    symbol: "ETH",
    denom: "bridge/eth",
    decimals: 18,
    coingeckoId: "ethereum",
  },
  "bridge/xrp": {
    type: "native",
    name: "Ripple",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/xrpl/images/xrp.svg",
    symbol: "XRP",
    denom: "bridge/xrp",
    decimals: 6,
    coingeckoId: "ripple",
  },
  "bridge/usdc": {
    type: "alloyed",
    name: "Ethereum USD Circle",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
    symbol: "USDC",
    denom: "bridge/usdc",
    decimals: 6,
    coingeckoId: "usd-coin",
  },
  "bridge/sol": {
    type: "native",
    name: "Solana",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/solana/images/sol.svg",
    symbol: "SOL",
    denom: "bridge/sol",
    decimals: 9,
    coingeckoId: "solana",
  },
} as const;

export const coinsBySymbol = Object.values(coins).reduce((acc, coin) => {
  acc[coin.symbol] = coin;
  return acc;
}, Object.create({}));

export const config: Config = createConfig({
  version: 0.4,
  multiInjectedProviderDiscovery: true,
  chain,
  transport: graphql(chain.urls.indexer, { batch: true }),
  coins,
  connectors: [passkey(), session()],
});
