import { createConfig, graphql, passkey, privy, session } from "@left-curve/store";
import { captureException } from "@sentry/react";

import type { Config } from "@left-curve/store/types";
import { serializeJson } from "@left-curve/dango/encoding";

import { PRIVY_APP_ID, PRIVY_CLIENT_ID } from "~/constants";

const chain = window.dango.chain;

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
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/refs/heads/master/_non-cosmos/bitcoin/images/btc.svg",
    symbol: "BTC",
    denom: "bridge/btc",
    decimals: 8,
    coingeckoId: "bitcoin",
  },
  "bridge/eth": {
    type: "native",
    name: "Ethereum",
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
    type: "native",
    name: "USD Coin",
    logoURI:
      "https://raw.githubusercontent.com/cosmos/chain-registry/master/axelar/images/usdc.svg",
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

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chain,
  transport: graphql(`${chain.urls.indexer}/graphql`, { batch: true }),
  coins,
  connectors: [
    passkey(),
    session(),
    privy({
      appId: PRIVY_APP_ID as string,
      clientId: PRIVY_CLIENT_ID as string,
      loadIframe: true,
    }),
  ],
  onError: (e) => {
    let finalError: Error;
    const m = serializeJson(e);

    if (Array.isArray(e) && e[0]?.message) {
      finalError = new Error(`GraphQLWS Error: ${e[0].message} (${m})`);
    } else if (e instanceof Event) {
      if ("code" in e) {
        finalError = new Error(`WebSocket closed: (${m})`);
      } else {
        finalError = new Error(`WebSocket connection failed (${m})`);
      }
    } else if (e instanceof Error) {
      finalError = e;
    } else {
      finalError = new Error(`Unknown Error (${m})`);
    }

    captureException(finalError);
  },
});
