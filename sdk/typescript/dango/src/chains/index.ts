import type { Chain } from "@left-curve/types";

import devnetJson from "./definitions/devnet.json" with { type: "json" };
import localJson from "./definitions/local.json" with { type: "json" };
import mainnetJson from "./definitions/mainnet.json" with { type: "json" };
import testnetJson from "./definitions/testnet.json" with { type: "json" };

export const devnet = devnetJson as Chain;
export const local = localJson as Chain;
export const mainnet = mainnetJson as Chain;
export const testnet = testnetJson as Chain;
