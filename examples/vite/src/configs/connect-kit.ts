import { http, createConfig, passkey } from "@leftcurve/connect-kit";
import { localhost } from "@leftcurve/connect-kit/chains";

export const config = createConfig({
  chains: [localhost],
  transports: {
    [localhost.id]: http("http://localhost:26657"),
  },
  connectors: [passkey()],
});
