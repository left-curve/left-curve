import { GrugProvider as Provider } from "@leftcurve/react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import type { PropsWithChildren } from "react";

import { http, createConfig, eip1193, passkey } from "@leftcurve/connect-kit";
import { localhost } from "@leftcurve/connect-kit/chains";
import "@leftcurve/types/window";

export const config = createConfig({
  chains: [localhost],
  transports: {
    [localhost.id]: http("http://localhost:26657", { batch: true }),
  },
  coins: {
    [localhost.id]: {
      uusdc: {
        type: "native",
        name: "USD Circle",
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

export const GrugProvider: React.FC<PropsWithChildren> = ({ children }) => {
  return (
    <Provider config={config}>
      <QueryClientProvider client={new QueryClient()}>{children}</QueryClientProvider>
    </Provider>
  );
};
