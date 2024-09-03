import { http, createConfig, eip1193, passkey } from "@leftcurve/connect-kit";
import { localhost } from "@leftcurve/connect-kit/chains";
import "@leftcurve/types/window";

export const config = createConfig({
  chains: [localhost],
  transports: {
    [localhost.id]: http("http://localhost:26657"),
  },
  connectors: [
    eip1193(), // Metamask
    eip1193({
      id: "keplr",
      name: "Keplr",
      icon: "/keplr.png",
      provider: () => window.keplr?.ethereum,
    }),
    passkey({
      icon: "/passkey-white.svg",
    }),
  ],
});
