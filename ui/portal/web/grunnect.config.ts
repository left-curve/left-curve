import { http, createConfig, devnet, passkey } from "@left-curve/store-react";

import type { Config } from "@left-curve/store-react/types";

const dango = devnet;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: true,
  chains: [dango],
  transports: {
    [dango.id]: http(dango.rpcUrls.default.http.at(0), { batch: true }),
  },
  coins: {
    [dango.id]: {
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
  connectors: [passkey()],
});
