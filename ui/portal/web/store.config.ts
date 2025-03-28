import { createConfig, devnet, graphql, passkey, session } from "@left-curve/store";

import type { Config } from "@left-curve/store/types";

const dango = devnet;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain: dango,
  transport: graphql(dango.urls.indexer, { batch: true }),
  coins: {
    [dango.id]: {
      ubtc: {
        type: "native",
        name: "Bitcoin",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg",
        symbol: "BTC",
        denom: "ubtc",
        decimals: 18,
        coingeckoId: "bitcoin",
      },
      ueth: {
        type: "native",
        name: "Ether",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/ethereum/images/eth.svg",
        symbol: "ETH",
        denom: "ueth",
        decimals: 18,
        coingeckoId: "ethereum",
      },
      uripple: {
        type: "native",
        name: "Ripple",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/xrpl/images/xrp.svg",
        symbol: "XRP",
        denom: "uripple",
        decimals: 18,
        coingeckoId: "ripple",
      },
      "hyp/eth/usdc": {
        type: "native",
        name: "Ethereum USD Circle",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
        symbol: "EUSDC",
        denom: "hyp/eth/usdc",
        decimals: 6,
        coingeckoId: "usd-coin",
      },
      usol: {
        type: "native",
        name: "Solana",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/solana/images/sol.svg",
        symbol: "SOL",
        denom: "usol",
        decimals: 18,
        coingeckoId: "solana",
      },
      uusdc: {
        type: "native",
        name: "USD Circle",
        logoURI:
          "https://raw.githubusercontent.com/cosmos/chain-registry/master/noble/images/USDCoin.svg",
        symbol: "USDC",
        denom: "uusdc",
        decimals: 6,
        coingeckoId: "usd-coin",
      },
    },
  },
  connectors: [passkey(), session()],
});
