"use client";

import { http, createConfig, eip1193, passkey } from "@left-curve/connect-kit";
import { devnet } from "@left-curve/connect-kit/chains";
import { GrunnectProvider } from "@left-curve/react";
import type React from "react";
import "@left-curve/types/window";

export const config = createConfig({
  ssr: true,
  multiInjectedProviderDiscovery: false,
  chains: [devnet],
  transports: {
    [devnet.id]: http("http://localhost:26657"),
  },
  coins: {
    [devnet.id]: {
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
  connectors: [
    eip1193({
      id: "metamask",
      name: "Metamask",
    }),
    eip1193({
      id: "keplr",
      name: "Keplr",
      provider: () => window.keplr?.ethereum,
    }),
    passkey(),
  ],
});

export interface ProvidersProps {
  children: React.ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  return <GrunnectProvider config={config}>{children}</GrunnectProvider>;
}
