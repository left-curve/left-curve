import { http, createConfig, passkey } from "@leftcurve/connect-kit";
import { devnet } from "@leftcurve/connect-kit/chains";
import "@leftcurve/types/window";

const dango = devnet;

export const config = createConfig({
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
