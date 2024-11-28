import { http, GrunnectProvider, createConfig, passkey } from "@left-curve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";

import { devnet } from "@left-curve/react/chains";

export const config = createConfig({
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

export const AppProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <GrunnectProvider config={config}>
      {/* "@tanstack/react-query" is required in combination with GrunnectProvider */}
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    </GrunnectProvider>
  );
};
