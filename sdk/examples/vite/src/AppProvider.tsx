import { GrunnectProvider as Provider } from "@left-curve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";

import { http, createConfig, passkey } from "@left-curve/connect-kit";
import { devnet } from "@left-curve/connect-kit/chains";
import "@left-curve/types/window";

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
    <Provider config={config}>
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    </Provider>
  );
};
