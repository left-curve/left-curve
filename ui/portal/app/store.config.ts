import { createConfig, createTransport, passkey, devnet, createStorage } from "@left-curve/store";

import { createMMKVStorage } from "./storage.config";
import { coins } from "@left-curve/foundation/coins";

import type { Config } from "@left-curve/store/types";

const chain = devnet;

export const config: Config = createConfig({
  multiInjectedProviderDiscovery: false,
  chain,
  transport: createTransport(chain.url, { batch: true }),
  coins,
  storage: createStorage({ storage: createMMKVStorage() }),
  connectors: [passkey()],
});
