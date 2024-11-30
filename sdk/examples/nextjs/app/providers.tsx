"use client";

import { http, GrunnectProvider, createConfig, passkey } from "@left-curve/react";
import { devnet } from "@left-curve/react/chains";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";

import type React from "react";

export const config = createConfig({
  ssr: true,
  multiInjectedProviderDiscovery: true,
  chains: [devnet],
  transports: {
    [devnet.id]: http(devnet.rpcUrls.default.http.at(0), { batch: true }),
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
  connectors: [passkey()],
});

export interface ProvidersProps {
  children: React.ReactNode;
}

export function Providers({ children }: ProvidersProps) {
  return (
    <GrunnectProvider config={config}>
      {/* "@tanstack/react-query" is required in combination with GrunnectProvider */}
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    </GrunnectProvider>
  );
}
